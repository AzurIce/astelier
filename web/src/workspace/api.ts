// 图文档与分组的存储接口。委托当前工作区 store（OPFS 或远端服务），
// 调用方（会话、侧栏、保存队列）不感知实现。
import type { GraphDoc, ViewDoc, GraphDocWithId, GraphGroup, GraphSummary } from './types'
import { workspaceStore } from './store'

export async function fetchGraph(id: string): Promise<GraphDocWithId> {
	return workspaceStore().fetchGraph(id)
}

export async function putGraph(id: string, doc: GraphDoc) {
	await workspaceStore().putGraph(id, doc)
}

/** 图重命名。OPFS 返回稳定 id；远端服务可能因目录改名返回新 id。 */
export async function renameGraph(id: string, title: string): Promise<{ id: string }> {
	return workspaceStore().renameGraph(id, title)
}

export async function fetchView(id: string): Promise<ViewDoc> {
	return workspaceStore().fetchView(id)
}

export async function putView(id: string, view: ViewDoc) {
	await workspaceStore().putView(id, view)
}

// ---------- 目录树（文件夹 + 图） ----------

export async function fetchGraphs(): Promise<GraphSummary[]> {
	return workspaceStore().listGraphs()
}

export async function fetchGroups(): Promise<GraphGroup[]> {
	return workspaceStore().listGroups()
}

export async function createGroup(name: string, parent_id: string | null): Promise<GraphGroup> {
	return workspaceStore().createGroup(name, parent_id)
}

export async function renameGroup(id: string, name: string) {
	await workspaceStore().renameGroup(id, name)
}

export async function moveGroup(id: string, parent_id: string | null) {
	await workspaceStore().moveGroup(id, parent_id)
}

export async function deleteGroup(id: string) {
	await workspaceStore().deleteGroup(id)
}

/** 建图到指定目录（null = 根） */
export async function createGraph(group_id: string | null = null, title?: string): Promise<GraphDocWithId> {
	return workspaceStore().createGraph(group_id, title)
}

export async function setGraphGroup(id: string, group_id: string | null) {
	await workspaceStore().setGraphGroup(id, group_id)
}

export async function deleteGraph(id: string) {
	await workspaceStore().deleteGraph(id)
}
