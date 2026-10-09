import type { BackendConfig } from './types'

export const LOCAL_BACKEND_ID = 'local'
export const LOCAL_BACKEND: BackendConfig = { id: LOCAL_BACKEND_ID, name: '本地', kind: 'opfs' }

export function normalizeServerUrl(raw: string): string {
	const url = new URL(raw.trim())
	if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) throw new Error('请输入 HTTP 或 HTTPS 服务地址')
	return url.href.replace(/\/+$/, '')
}

export function parseConnections(raw: string | null): BackendConfig[] {
	const result = [{ ...LOCAL_BACKEND }]
	try {
		const entries: unknown = JSON.parse(raw ?? '[]')
		if (!Array.isArray(entries)) return result
		for (const entry of entries) {
			if (entry?.kind === 'opfs' && entry.id === LOCAL_BACKEND_ID && typeof entry.name === 'string' && entry.name.trim()) { result[0].name = entry.name.trim(); continue }

			if (entry?.kind !== 'http' || typeof entry.id !== 'string' || !entry.id || entry.id === LOCAL_BACKEND_ID || typeof entry.baseUrl !== 'string') continue
			try {
				const baseUrl = normalizeServerUrl(entry.baseUrl)
				if (result.some((item) => item.id === entry.id || item.baseUrl === baseUrl)) continue
				result.push({ id: entry.id, kind: 'http', baseUrl, name: typeof entry.name === 'string' && entry.name.trim() ? entry.name.trim() : baseUrl })
			} catch { /* Invalid persisted connection is ignored. */ }
		}
	} catch { /* Start with local storage if device preferences are corrupt. */ }
	return result
}

export async function serverIdentity(baseUrl: string): Promise<string> {
	const res = await fetch(`${baseUrl}/api/backend`, { signal: AbortSignal.timeout(15_000) })
	if (!res.ok) throw new Error(`读取后端身份失败（HTTP ${res.status}）`)
	const info = await res.json()
	if (typeof info?.id !== 'string' || !info.id || info.id === LOCAL_BACKEND_ID) throw new Error('服务端未提供有效后端身份')
	return info.id
}
