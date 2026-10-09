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

export type GraphDocWithId = GraphDoc & { id: string; title?: string; group_id?: string | null; created_at?: number; updated_at?: number }

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
