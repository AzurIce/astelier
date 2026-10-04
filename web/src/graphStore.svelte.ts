// 画布持久化：结构文档 + 表现文档分别节流保存到服务端。
// localStorage 只保留「当前图 id」。首访无图时由服务端建种子图。
import {
	toDoc,
	toViewDoc,
	loadDoc,
	type ViewDoc,
} from './graphDoc'
import { fetchView, createGraph, fetchGraph, putGraph, putView, renameGraph, type GraphDocWithId } from './api'
import { rt, runningNodes } from './runtime'

import { SaveQueue, type SaveState } from './saveQueue'
export type { SaveState } from './saveQueue'

const KEY = 'atelier-graph-id'

/** 顶栏、侧栏和编辑器读取同一份活动图状态。 */
export const graphSession = $state({
	id: '',
	title: '未命名图',
	saveState: 'saved' as SaveState,
	loading: false,
})

let restoring = false
const saves = new SaveQueue(async (kind) => {
	const id = graphSession.id
	if (!id || !rt.editor || !rt.area) return
	if (kind === 'graph') await putGraph(id, toDoc())
	else await putView(id, toViewDoc())
}, (state) => { graphSession.saveState = state })

async function restore(doc: GraphDocWithId, view?: ViewDoc): Promise<void> {
	restoring = true
	try {
		runningNodes.clear()
		await rt.editor?.clear()
		setActiveGraphId(doc.id)
		graphSession.title = doc.title ?? '未命名图'
		await loadDoc(doc, view)
		graphSession.saveState = 'saved'
	} finally {
		restoring = false
	}
}

/** 解析当前图（本地记录优先，失效则建种子图）并载入画布。 */
export async function ensureGraphAndLoad(): Promise<GraphDocWithId> {
	graphSession.loading = true
	try {
		const stored = localStorage.getItem(KEY) ?? ''
		const doc = (stored ? await fetchGraph(stored).catch(() => null) : null) ?? await createGraph()
		const view = await fetchView(doc.id).catch(() => undefined)
		await restore(doc, view)
		return doc
	} finally {
		graphSession.loading = false
	}
}

export function activeGraphId(): string {
	return graphSession.id
}

/** 图目录改名后同步活动记录；标题与 ID 一起更新。 */
export function setActiveGraphId(id: string, title?: string): void {
	graphSession.id = id
	if (title !== undefined) graphSession.title = title
	localStorage.setItem(KEY, id)
}

export function flushNow(): Promise<void> {
	return saves.flush()
}

// 打开/改名按顺序处理，旧图写入结束后才能切换目录或节点实例。
let changing: Promise<void> = Promise.resolve()
function changeGraph(action: () => Promise<void>): Promise<void> {
	const next = changing.then(action)
	changing = next.catch(() => {})
	return next
}

export function openGraph(id: string): Promise<void> {
	return changeGraph(async () => {
		if (id === graphSession.id || !rt.editor || !rt.area) return
		graphSession.loading = true
		try {
			await flushNow()
			const doc = await fetchGraph(id)
			const view = await fetchView(id).catch(() => undefined)
			await restore(doc, view)
		} finally {
			graphSession.loading = false
		}
	})
}

/** 活动图的目录迁移必须等保存结束，并同步顶栏与侧栏的 ID/标题。 */
export function renameGraphAndSync(id: string, title: string): Promise<void> {
	return changeGraph(async () => {
		const active = id === graphSession.id
		if (active) graphSession.loading = true
		try {
			if (active) await flushNow()
			const renamed = await renameGraph(id, title)
			if (active) setActiveGraphId(renamed.id, title)
		} finally {
			if (active) graphSession.loading = false
		}
	})
}

/** 结构变更（节点/连线/参数）→ PUT 文档。载入过程不产生修改。 */
export function scheduleSave(): void {
	if (!restoring && graphSession.id) saves.schedule('graph', 400)
}

/** 表现变更（拖动/视口）→ PUT 文档。 */
export function scheduleViewSave(): void {
	if (!restoring && graphSession.id) saves.schedule('view', 600)
}

// ---------- 图私有 image store（内联感知：UI 不可见） ----------

export interface GraphStoreFileMeta {
	name: string
	w?: number
	h?: number
	bytes?: number
}

/** 图内 store 文件列表 */
export async function fetchGraphStore(gid: string): Promise<GraphStoreFileMeta[]> {
	const res = await fetch(`/api/graphs/${encodeURIComponent(gid)}/store`)
	if (!res.ok) throw new Error(`读取图 store 失败（${res.status}）`)
	return res.json()
}

/** 上传 / 复制进图内 store（文件名即引用） */
export async function uploadGraphStoreFile(
	gid: string,
	name: string,
	blob: Blob,
): Promise<GraphStoreFileMeta> {
	const res = await fetch(
		`/api/graphs/${encodeURIComponent(gid)}/store?filename=${encodeURIComponent(name)}`,
		{ method: 'POST', body: blob },
	)
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return body as GraphStoreFileMeta
}

/** 图内 store 静态 URL */
export function graphStoreUrl(gid: string, name: string): string {
	return `/gstore/${encodeURIComponent(gid)}/${encodeURIComponent(name)}`
}
