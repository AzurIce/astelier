// WorkspaceStore：工作区存储的抽象边界。
// 调用方（图文档、侧栏、图片库、图内参考图、配置）只依赖这里的接口，
// 不感知数据来自 OPFS 还是远端服务。实例由 main.ts 装配；测试通过
// window.__atelierRuntime 注入替换实现。
import type { GraphDoc, GraphDocWithId, GraphGroup, GraphSummary, ProviderConfig, StoreFileEntry, StoreTree, ViewDoc } from './types'
import type { GraphStoreFileMeta } from '../images/types'

export interface WorkspaceStore {
	/** 工作区形态（UI 展示与装配判断用） */
	readonly kind: 'opfs' | 'http'
	readonly label: string

	// ---------- 配置 ----------
	loadConfig(): Promise<ProviderConfig>
	saveConfig(config: ProviderConfig): Promise<void>

	// ---------- 图文档 ----------
	listGraphs(): Promise<GraphSummary[]>
	createGraph(groupId: string | null, title?: string): Promise<GraphDocWithId>
	fetchGraph(id: string): Promise<GraphDocWithId>
	putGraph(id: string, doc: GraphDoc): Promise<void>
	/** 重命名。OPFS 返回稳定 id；HTTP 实现可能返回目录改名后的新 id。 */
	renameGraph(id: string, title: string): Promise<{ id: string }>
	fetchView(id: string): Promise<ViewDoc>
	putView(id: string, view: ViewDoc): Promise<void>
	setGraphGroup(id: string, groupId: string | null): Promise<void>
	deleteGraph(id: string): Promise<void>

	// ---------- 图分组 ----------
	listGroups(): Promise<GraphGroup[]>
	createGroup(name: string, parentId: string | null): Promise<GraphGroup>
	renameGroup(id: string, name: string): Promise<void>
	moveGroup(id: string, parentId: string | null): Promise<void>
	deleteGroup(id: string): Promise<void>

	// ---------- 图内参考图 ----------
	uploadGraphStoreFile(gid: string, name: string, blob: Blob): Promise<GraphStoreFileMeta>
	/** 展示/拖拽用 URL：OPFS 为缓存 blob: URL（文件缺失返回 null）；HTTP 为绝对地址 */
	graphStoreUrl(gid: string, name: string): Promise<string | null>
	deleteGraphStoreFile(gid: string, name: string): Promise<void>

	// ---------- 图片库 ----------
	storeTree(): Promise<StoreTree>
	uploadStoreFile(file: File, dir: string): Promise<StoreFileEntry>
	makeStoreDir(path: string): Promise<void>
	moveStorePath(from: string, to: string): Promise<void>
	deleteStorePath(path: string): Promise<void>
	storeUrl(path: string): Promise<string | null>
}

let current: WorkspaceStore | null = null

export function setWorkspaceStore(store: WorkspaceStore): void {
	current = store
}

export function workspaceStore(): WorkspaceStore {
	if (!current) throw new Error('工作区未初始化（应由 main.ts 装配）')
	return current
}
