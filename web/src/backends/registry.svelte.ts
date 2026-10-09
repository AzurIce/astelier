import { createOpfsStore } from '../workspace/opfs/store'
import { createHttpStore } from '../workspace/httpStore'
import type { WorkspaceStore } from '../workspace/store'
import { loadLocalConfig } from '../generation/localConfig'
import { createDirectGenerator } from '../generation/direct'
import { createRemoteGenerator } from '../generation/remote'
import type { ImageGenerator } from '../generation/generator'
import { mergedProfile } from '../generation/profiles'
import { CONNECTIONS_KEY, LOCAL_BACKEND_ID, normalizeServerUrl, parseConnections, serverIdentity } from './connections'
import type { BackendConfig, BackendConnection, ProviderDescriptor, ProviderLocation } from './types'

export const backendRegistry = $state<{ entries: BackendConnection[] }>({ entries: [] })
const runtimes = new Map<string, { store: WorkspaceStore; generators: Map<string, ImageGenerator> }>()
const pending = new Map<string, Promise<void>>()
const overrides = () => (window as { __atelierRuntime?: { wrapStore?: (store: WorkspaceStore, backendId: string) => WorkspaceStore; generate?: ImageGenerator['generate'] } }).__atelierRuntime

function register(config: BackendConfig): void {
	const store = config.kind === 'opfs' ? createOpfsStore() : createHttpStore(config.baseUrl!)
	runtimes.set(config.id, { store: overrides()?.wrapStore?.(store, config.id) ?? store, generators: new Map() })
	backendRegistry.entries.push({ ...config, status: 'connecting', error: null, providerError: null, providers: [], revision: 0 })
}

export function initializeBackends(): void {
	if (runtimes.size) return
	for (const config of parseConnections(localStorage.getItem(CONNECTIONS_KEY))) register(config)
}

export function backend(id: string): BackendConnection {
	const entry = backendRegistry.entries.find((entry) => entry.id === id)
	if (!entry) throw new Error('后端连接已移除')
	return entry
}

export function backendStore(id: string): WorkspaceStore {
	const runtime = runtimes.get(id)
	if (!runtime) throw new Error('后端连接已移除')
	return runtime.store
}

function persistConnections(): void {
	localStorage.setItem(CONNECTIONS_KEY, JSON.stringify(backendRegistry.entries.map(({ id, name, kind, baseUrl }) => ({ id, name, kind, baseUrl }))))
}

export async function refreshProviders(id: string): Promise<void> {
	const entry = backend(id), runtime = runtimes.get(id)!
	try {
		let providers: ProviderDescriptor[]
		const generators = new Map<string, ImageGenerator>()
		if (entry.kind === 'opfs') {
			const config = await loadLocalConfig()
			providers = config.providers.map((provider) => {
				generators.set(provider.id, createDirectGenerator(structuredClone(provider)))
				return { id: provider.id, name: provider.name, models: provider.models, profiles: Object.fromEntries(provider.models.map((model) => [model, mergedProfile(model, provider.overrides?.[model])])) }
			})
		} else {
			const res = await fetch(`${entry.baseUrl}/api/providers`, { signal: AbortSignal.timeout(15_000) })
			if (!res.ok) throw new Error(`读取 Provider 失败（HTTP ${res.status}）`)
			providers = await res.json()
			if (!Array.isArray(providers) || providers.some((p) => typeof p.id !== 'string' || !Array.isArray(p.models) || !p.profiles)) throw new Error('Provider 列表格式不正确')
			for (const provider of providers) generators.set(provider.id, createRemoteGenerator(entry.baseUrl!, provider.id))
		}
		if (runtimes.get(id) !== runtime) return
		runtime.generators = generators
		entry.providers = providers
		entry.providerError = null
	} catch (error) {
		entry.providerError = error instanceof Error ? error.message : String(error)
		throw error
	}
}

export function connectBackend(id: string): Promise<void> {
	const existing = pending.get(id)
	if (existing) return existing
	const entry = backend(id), runtime = runtimes.get(id)!
	entry.status = 'connecting'
	const work = (async () => {
		try {
			if (entry.kind === 'http' && await serverIdentity(entry.baseUrl!) !== id) throw new Error('该地址上的后端身份已改变，请重新添加连接')
			await runtime.store.listGraphs()
			if (runtimes.get(id) !== runtime) return
			entry.status = 'online'
			entry.error = null
			await refreshProviders(id).catch(() => {})
		} catch (error) {
			entry.status = 'offline'
			entry.error = error instanceof Error ? error.message : String(error)
		} finally {
			entry.revision++
			pending.delete(id)
		}
	})()
	pending.set(id, work)
	return work
}

export async function addServer(name: string, rawUrl: string): Promise<string> {
	const baseUrl = normalizeServerUrl(rawUrl)
	if (backendRegistry.entries.some((entry) => entry.baseUrl === baseUrl)) throw new Error('该服务已经添加')
	const id = await serverIdentity(baseUrl)
	if (backendRegistry.entries.some((entry) => entry.id === id)) throw new Error('该后端已通过另一个地址连接')
	register({ id, kind: 'http', baseUrl, name: name.trim() || baseUrl })
	persistConnections()
	await connectBackend(id)
	return id
}

/** Caller drains the active graph before unmounting its backend. */
export function removeBackend(id: string): void {
	if (id === LOCAL_BACKEND_ID) throw new Error('本地后端不能移除')
	runtimes.delete(id)
	backendRegistry.entries = backendRegistry.entries.filter((entry) => entry.id !== id)
	persistConnections()
}

export function renameBackend(id: string, name: string): void {
	if (!name.trim()) throw new Error('名称不能为空')
	backend(id).name = name.trim()
	persistConnections()
}

export function generatorFor(location: ProviderLocation): ImageGenerator {
	const entry = backend(location.backendId)
	if (entry.status !== 'online' || entry.providerError) throw new Error(`Provider 来源「${entry.name}」暂不可用`)
	const generator = runtimes.get(entry.id)?.generators.get(location.providerId)
	if (!generator) throw new Error('所选 Provider 不存在或已移除')
	return overrides()?.generate ? { generate: overrides()!.generate! } : generator
}
