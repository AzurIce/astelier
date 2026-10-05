// 持久文档契约，不依赖 Rete、Svelte 或 HTTP 实现。
export type NodeType = 'model' | 'prompt' | 'image' | 'generate' | 'preview'

export interface DocNode {
	id: string
	type: NodeType
	params: Record<string, string | number | boolean | object | null>
}
export interface DocEdge {
	id: string
	source: string
	sourcePort: string
	target: string
	targetPort: string
}
export interface GraphDoc {
	version: 1
	nodes: DocNode[]
	edges: DocEdge[]
}
export interface ViewDoc {
	version: 1
	positions: Record<string, { x: number; y: number }>
	viewport?: { x: number; y: number; zoom: number }
}

export type GraphDocWithId = GraphDoc & { id: string; title?: string; group_id?: string | null }

export interface GraphGroup {
	id: string
	name: string
	parent_id: string | null
	created_at: number
	/** OPFS 实现写入（同步友好）；HTTP 实现没有该字段，宽容读取 */
	updated_at?: number
}

export interface GraphSummary {
	id: string
	title: string
	group_id: string | null
	updated_at: number
}

// ---------- Provider 配置（config.json） ----------

export interface ProviderEntry {
	id: string
	name: string
	/** 含 /v1 的根地址，如 https://api.openai.com/v1 */
	base_url: string
	/** 本地模式明文保存（可在设置里清除）；远端模式留在服务端 */
	api_key: string
	models: string[]
	/** 模型档案覆盖（model_id → ModelProfile 子集） */
	overrides: Record<string, unknown>
}

export interface ProviderConfig {
	providers: ProviderEntry[]
	active_provider: string
}

// ---------- 图片库（stores/） ----------

export interface StoreFileEntry {
	/** 相对路径，如 "角色/猫.png" */
	path: string
	w?: number
	h?: number
	bytes: number
}

export interface StoreTree {
	dirs: string[]
	files: StoreFileEntry[]
}
