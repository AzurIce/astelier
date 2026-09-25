// 画布持久化：结构文档 + 表现文档分别节流保存到服务端。
// localStorage 只保留「当前图 id」。首访无图时由服务端建种子图。
import {
	toDoc,
	toViewDoc,
	loadDoc,
	type ViewDoc,
} from './graphDoc'
import { fetchView, createGraph, fetchGraph, putGraph, putView, type GraphDocWithId } from './api'
import { rt } from './runtime'

const KEY = 'atelier-graph-id'

let graphId = ''
let structTimer: ReturnType<typeof setTimeout> | null = null
let viewTimer: ReturnType<typeof setTimeout> | null = null

/* ---------------- 保存状态广播（顶栏小圆点） ---------------- */

export type SaveState = 'saved' | 'dirty' | 'saving'

const saveSubs = new Set<(s: SaveState) => void>()
let saveState: SaveState = 'saved'

function setSaveState(s: SaveState) {
	if (s === saveState) return
	saveState = s
	for (const cb of saveSubs) cb(s)
}

/** 订阅保存状态；立即回调一次当前值 */
export function onSaveState(cb: (s: SaveState) => void): () => void {
	saveSubs.add(cb)
	cb(saveState)
	return () => saveSubs.delete(cb)
}

/** 解析当前图（本地记录的 id 优先，失效则由服务端建种子图）并载入画布 */
export async function ensureGraphAndLoad(): Promise<GraphDocWithId | null> {
	const stored = localStorage.getItem(KEY) ?? ''
	let doc: GraphDocWithId | null = null
	if (stored) {
		try {
			doc = await fetchGraph(stored)
		} catch {
			doc = null
		}
	}
	if (!doc) {
		doc = await createGraph()
	}
	graphId = doc.id
	localStorage.setItem(KEY, doc.id)

	let view: ViewDoc | undefined
	try {
		view = await fetchView(doc.id)
	} catch {
		view = undefined
	}
	await loadDoc(doc, view)
	return doc
}

export function activeGraphId(): string {
	return graphId
}

/** 图目录改名后 id 会变：同步本地的活动图记录 */
export function setActiveGraphId(id: string) {
	graphId = id
	localStorage.setItem(KEY, id)
}

/** 立即落盘未保存的挂起变更（切图前调用） */
export async function flushNow(): Promise<void> {
	if (structTimer) {
		clearTimeout(structTimer)
		structTimer = null
		await flushGraph()
	}
	if (viewTimer) {
		clearTimeout(viewTimer)
		viewTimer = null
		await flushView()
	}
}

/** 打开另一张图：落盘旧图 → 清空编辑器 → 载入新图文档与视图 */
export async function openGraph(id: string): Promise<void> {
	if (id === graphId || !rt.editor || !rt.area) return
	await flushNow()
	const doc = await fetchGraph(id)
	const view = await fetchView(id).catch(() => undefined)
	graphId = id
	localStorage.setItem(KEY, id)
	await rt.editor.clear()
	await loadDoc(doc, view)
}

/** 结构变更（节点/连线/参数）→ PUT 结构文档 */
export function scheduleSave() {
	if (structTimer) return
	structTimer = setTimeout(flushGraph, 400)
	setSaveState('dirty')
}

async function flushGraph() {
	structTimer = null
	if (!graphId || !rt.editor) return
	setSaveState('saving')
	try {
		await putGraph(graphId, toDoc())
		setSaveState('saved')
	} catch (e) {
		console.error('保存画布结构失败', e)
		setSaveState('saved')
	}
}

/** 表现变更（拖动/视口/产物）→ PUT 表现文档 */
export function scheduleViewSave() {
	if (viewTimer) return
	viewTimer = setTimeout(flushView, 600)
}

async function flushView() {
	viewTimer = null
	if (!graphId || !rt.editor) return
	try {
		await putView(graphId, toViewDoc())
	} catch (e) {
		console.error('保存画布视图失败', e)
	}
}
