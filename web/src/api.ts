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

export type GraphDocWithId = GraphDoc & { id: string; title?: string; group_id?: string | null }

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

/** 图重命名 = 目录改名，id 可能随之变化；返回新 id */
export async function renameGraph(id: string, title: string): Promise<{ id: string }> {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/title`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name: title }),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `重命名失败（${res.status}）`)
	return body as { id: string }
}

export async function fetchView(id: string): Promise<ViewDoc> {	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/view`)
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

// ---------- 目录树（文件夹 + 图） ----------

export interface GraphGroup {
	id: string
	name: string
	parent_id: string | null
	created_at: number
}

export interface GraphSummary {
	id: string
	title: string
	group_id: string | null
	updated_at: number
}

export async function fetchGraphs(): Promise<GraphSummary[]> {
	const res = await fetch('/api/graphs')
	if (!res.ok) throw new Error(`读取图列表失败（${res.status}）`)
	return res.json()
}

export async function fetchGroups(): Promise<GraphGroup[]> {
	const res = await fetch('/api/groups')
	if (!res.ok) throw new Error(`读取目录失败（${res.status}）`)
	return res.json()
}

export async function createDir(name: string, parent_id: string | null): Promise<GraphGroup> {
	const res = await fetch('/api/groups', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name, parent_id }),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `新建文件夹失败（${res.status}）`)
	return body as GraphGroup
}

export async function renameGroup(id: string, name: string) {
	const res = await fetch(`/api/groups/${encodeURIComponent(id)}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name }),
	})
	if (!res.ok) throw new Error(`重命名失败（${res.status}）`)
}

export async function moveGroup(id: string, parent_id: string | null) {
	const res = await fetch(`/api/groups/${encodeURIComponent(id)}/parent`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ parent_id }),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `移动失败（${res.status}）`)
}

export async function deleteGroup(id: string) {
	const res = await fetch(`/api/groups/${encodeURIComponent(id)}`, { method: 'DELETE' })
	if (!res.ok) throw new Error(`删除目录失败（${res.status}）`)
}

/** 建图到指定目录（null = 根） */
export async function createGraphIn(group_id: string | null, title?: string): Promise<GraphDocWithId> {
	const res = await fetch('/api/graphs', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ group_id, title }),
	})
	if (!res.ok) throw new Error(`建图失败（${res.status}）`)
	return res.json()
}

export async function setGraphGroup(id: string, group_id: string | null) {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/group`, {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ group_id }),
	})
	if (!res.ok) throw new Error(`移动图失败（${res.status}）`)
}

export async function deleteGraph(id: string) {
	const res = await fetch(`/api/graphs/${encodeURIComponent(id)}`, { method: 'DELETE' })
	if (!res.ok) throw new Error(`删除图失败（${res.status}）`)
}
