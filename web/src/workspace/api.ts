// 工作区图文档与分组的 HTTP 存储接口。
import type { GraphDoc, ViewDoc, GraphDocWithId, GraphGroup, GraphSummary } from './types'

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

// ---------- 目录树（文件夹 + 图） ----------

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

export async function createGroup(name: string, parent_id: string | null): Promise<GraphGroup> {
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
export async function createGraph(group_id: string | null = null, title?: string): Promise<GraphDocWithId> {
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
