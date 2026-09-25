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

/** 解析当前图（本地记录的 id 优先，失效则由服务端建种子图）并载入画布 */
export async function ensureGraphAndLoad(): Promise<void> {
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
}

/** 结构变更（节点/连线/参数）→ PUT 结构文档 */
export function scheduleSave() {
	if (structTimer) return
	structTimer = setTimeout(flushGraph, 400)
}

async function flushGraph() {
	structTimer = null
	if (!graphId || !rt.editor) return
	try {
		await putGraph(graphId, toDoc())
	} catch (e) {
		console.error('保存画布结构失败', e)
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
