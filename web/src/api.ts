// 后端 REST API 封装（错误统一抛 Error(message)）

export interface ProviderInfo {
	id: string
	name: string
	models: string[]
}

export async function fetchConfig(): Promise<{
	providers: ProviderInfo[]
	active_provider: string
}> {
	const res = await fetch('/api/config')
	if (!res.ok) throw new Error(`读取配置失败（${res.status}）`)
	return res.json()
}

export interface GenerateParams {
	model: string
	prompt: string
	/** 统一键 → 标量（数字/字符串）；服务端按模型档案过滤与校验 */
	params?: Record<string, string | number>
	imageUrls?: string[]
}

export interface GenerateResult {
	imageUrl: string
}

export async function apiGenerate(params: GenerateParams): Promise<GenerateResult> {
	const res = await fetch('/api/generate', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) {
		throw new Error(body?.error ?? `生成失败（${res.status}）`)
	}
	return body as GenerateResult
}

export async function uploadAsset(file: File): Promise<string> {
	const res = await fetch(`/api/assets?filename=${encodeURIComponent(file.name)}`, {
		method: 'POST',
		body: file,
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return `/asset/${body.id}.${body.ext}`
}

// ---------- 画布文档 ----------

import type { GraphDoc, ViewDoc } from './graphDoc'

export type GraphDocWithId = GraphDoc & { id: string }

export async function createGraph(): Promise<GraphDocWithId> {
	const res = await fetch('/api/graphs', { method: 'POST' })
	if (!res.ok) throw new Error(`建图失败（${res.status}）`)
	return res.json()
}

export async function fetchGraph(id: string): Promise<GraphDocWithId> {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}`)
	if (!res.ok) throw new Error(`读取图失败（${res.status}）`)
	return res.json()
}

export async function putGraph(id: string, doc: GraphDoc) {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ ...doc, id }),
	})
	if (!res.ok) throw new Error(`保存图失败（${res.status}）`)
}

export async function fetchView(id: string): Promise<ViewDoc> {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/view`)
	if (!res.ok) throw new Error(`读取视图失败（${res.status}）`)
	return res.json()
}

export async function putView(id: string, view: ViewDoc) {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/view`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(view),
	})
	if (!res.ok) throw new Error(`保存视图失败（${res.status}）`)
}
