// WorkspaceStore 的远端 HTTP 实现：对接现有 Rust 服务 /api/*，
// 图片 URL 为服务端绝对地址；
// 密钥、档案校验与参考图解析都在服务端完成。
import type { GraphDoc, GraphDocWithId, GraphGroup, GraphSummary, StoreFileEntry, StoreTree, ViewDoc } from './types'
import type { WorkspaceStore } from './store'
import type { GraphStoreFileMeta } from '../images/types'

function encodeStorePath(path: string): string {
	return path
		.split('/')
		.map((segment) => encodeURIComponent(segment))
		.join('/')
}

/** 返回普通对象字面量（与 OPFS 实现一致，便于测试包裹） */
export function createHttpStore(rawBase: string): WorkspaceStore {
	const base = rawBase.trim().replace(/\/+$/, '')

	async function api<T>(path: string, init?: RequestInit): Promise<T> {
		const res = await fetch(`${base}${path}`, { signal: AbortSignal.timeout(60_000), ...init })
		const body = await res.json().catch(() => null)
		if (!res.ok) throw new Error((body as { error?: string })?.error ?? `请求失败（${res.status}）`)
		return body as T
	}

	const jsonInit = (method: string, payload: unknown): RequestInit => ({
		method,
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(payload),
	})

	return {
		kind: 'http',
		label: `远端工作区（${base}）`,

		async listGraphs(): Promise<GraphSummary[]> {
			return api<GraphSummary[]>('/api/graphs')
		},

		async createGraph(groupId, title) {
			return api<GraphDocWithId>('/api/graphs', jsonInit('POST', { group_id: groupId, title }))
		},

		async fetchGraph(id): Promise<GraphDocWithId> {
			return api<GraphDocWithId>(`/api/graphs/${encodeURIComponent(id)}`)
		},

		async putGraph(id, doc: GraphDoc) {
			await api(`/api/graphs/${encodeURIComponent(id)}`, jsonInit('PUT', { ...doc, id }))
		},

		async renameGraph(id, title) {
			// 图身份与标题独立
			await api(`/api/graphs/${encodeURIComponent(id)}/title`, jsonInit('PUT', { name: title }))
		},

		async fetchView(id): Promise<ViewDoc> {
			const view = await api<Partial<ViewDoc>>(`/api/graphs/${encodeURIComponent(id)}/view`)
			return {
				version: 1,
				positions: view?.positions ?? {},
				...(view?.viewport ? { viewport: view.viewport } : {}),
			}
		},

		async putView(id, view) {
			await api(`/api/graphs/${encodeURIComponent(id)}/view`, jsonInit('PUT', view))
		},

		async setGraphGroup(id, groupId) {
			await api(`/api/graphs/${encodeURIComponent(id)}/group`, jsonInit('PUT', { group_id: groupId }))
		},

		async deleteGraph(id) {
			await api(`/api/graphs/${encodeURIComponent(id)}`, { method: 'DELETE' })
		},

		async listGroups(): Promise<GraphGroup[]> {
			return api<GraphGroup[]>('/api/groups')
		},

		async createGroup(name, parentId) {
			return api<GraphGroup>('/api/groups', jsonInit('POST', { name, parent_id: parentId }))
		},

		async renameGroup(id, name) {
			await api(`/api/groups/${encodeURIComponent(id)}`, jsonInit('PATCH', { name }))
		},

		async moveGroup(id, parentId) {
			await api(`/api/groups/${encodeURIComponent(id)}/parent`, jsonInit('PATCH', { parent_id: parentId }))
		},

		async deleteGroup(id) {
			await api(`/api/groups/${encodeURIComponent(id)}`, { method: 'DELETE' })
		},

		async uploadGraphStoreFile(gid, name, blob): Promise<GraphStoreFileMeta> {
			return api<GraphStoreFileMeta>(
				`/api/graphs/${encodeURIComponent(gid)}/store?filename=${encodeURIComponent(name)}`,
				{ method: 'POST', body: blob },
			)
		},

		async graphStoreUrl(gid, name) {
			return `${base}/gstore/${encodeURIComponent(gid)}/${encodeURIComponent(name)}`
		},

		async deleteGraphStoreFile(gid, name) {
			await api(`/api/graphs/${encodeURIComponent(gid)}/store/${encodeURIComponent(name)}`, { method: 'DELETE' })
		},

		async storeTree(): Promise<StoreTree> {
			return api<StoreTree>('/api/stores')
		},

		async uploadStoreFile(file, dir): Promise<StoreFileEntry> {
			return api<StoreFileEntry>(
				`/api/stores?filename=${encodeURIComponent(file.name)}&dir=${encodeURIComponent(dir)}`,
				{ method: 'POST', body: file },
			)
		},

		async makeStoreDir(path) {
			await api('/api/stores/dirs', jsonInit('POST', { path }))
		},

		async moveStorePath(from, to) {
			await api(`/api/stores/${encodeStorePath(from)}`, jsonInit('PATCH', { to }))
		},

		async deleteStorePath(path) {
			await api(`/api/stores/${encodeStorePath(path)}`, { method: 'DELETE' })
		},

		async storeUrl(path) {
			return `${base}/store/${encodeStorePath(path)}`
		},
	}
}
