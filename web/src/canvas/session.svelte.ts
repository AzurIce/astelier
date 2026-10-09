import { toDoc, toViewDoc, loadDoc } from './document'
import type { ViewDoc, GraphDocWithId } from '../workspace/types'
import type { GraphLocation } from '../backends/types'
import { backendStore, backend, connectBackend, removeBackend } from '../backends/registry.svelte'
import { LOCAL_BACKEND_ID } from '../backends/connections'
import { rt, runningNodes } from './runtime'
import { SaveQueue, type SaveState } from './saveQueue'

const KEY = 'atelier-active-graph'
export const graphSession = $state({ backendId: LOCAL_BACKEND_ID, id: '', title: '未命名图', saveState: 'saved' as SaveState, loading: false, epoch: 0 })
let restoring = false

// Capture destination before asynchronous writes; switching waits for the queue.
const saves = new SaveQueue(async (kind) => {
	const { backendId, id } = graphSession
	if (!id || !rt.editor || !rt.area) return
	const store = backendStore(backendId)
	if (kind === 'graph') await store.putGraph(id, toDoc())
	else await store.putView(id, toViewDoc())
}, (state) => { graphSession.saveState = state })

export function activeGraph(): GraphLocation { return { backendId: graphSession.backendId, id: graphSession.id } }
export function isCurrentGraph(location: GraphLocation): boolean { return location.backendId === graphSession.backendId && location.id === graphSession.id }

function setActiveGraph(location: GraphLocation, title: string): void {
	graphSession.backendId = location.backendId
	graphSession.id = location.id
	graphSession.title = title
	localStorage.setItem(KEY, JSON.stringify(location))
}

async function restore(backendId: string, doc: GraphDocWithId, view?: ViewDoc): Promise<void> {
	restoring = true
	graphSession.epoch++
	try {
		runningNodes.clear()
		await rt.editor?.clear()
		setActiveGraph({ backendId, id: doc.id }, doc.title ?? '未命名图')
		await loadDoc(doc, view)
		graphSession.saveState = 'saved'
	} finally { restoring = false }
}

export async function ensureGraphAndLoad(): Promise<GraphDocWithId> {
	graphSession.loading = true
	try {
		let location: GraphLocation | null = null
		try {
			const parsed = JSON.parse(localStorage.getItem(KEY) ?? 'null')
			if (typeof parsed?.backendId === 'string' && typeof parsed?.id === 'string') { backend(parsed.backendId); location = parsed }
		} catch { /* Invalid preferences use local storage. */ }
		let doc: GraphDocWithId | null = null
		if (location) {
			await connectBackend(location.backendId)
			if (backend(location.backendId).status === 'online') doc = await backendStore(location.backendId).fetchGraph(location.id).catch(() => null)
		}
		const backendId = doc && location ? location.backendId : LOCAL_BACKEND_ID
		const store = backendStore(backendId)
		if (!doc) {
			const first = (await store.listGraphs())[0]
			doc = first ? await store.fetchGraph(first.id) : await store.createGraph(null)
		}
		await restore(backendId, doc, await store.fetchView(doc.id))
		return doc
	} finally { graphSession.loading = false }
}

export function flushNow(): Promise<void> { return saves.flush() }
let changing: Promise<void> = Promise.resolve()
function changeGraph(action: () => Promise<void>): Promise<void> {
	const next = changing.then(action)
	changing = next.catch(() => {})
	return next
}

export function openGraph(location: GraphLocation): Promise<void> {
	return changeGraph(async () => {
		if (isCurrentGraph(location) || !rt.editor || !rt.area) return
		graphSession.loading = true
		try {
			await flushNow()
			const store = backendStore(location.backendId)
			const doc = await store.fetchGraph(location.id)
			await restore(location.backendId, doc, await store.fetchView(doc.id))
		} finally { graphSession.loading = false }
	})
}

export function renameGraphAndSync(location: GraphLocation, title: string): Promise<void> {
	return changeGraph(async () => {
		const active = isCurrentGraph(location)
		if (active) graphSession.loading = true
		try {
			if (active) await flushNow()
			await backendStore(location.backendId).renameGraph(location.id, title)
			if (active) setActiveGraph(location, title)
			backend(location.backendId).revision++
		} finally { if (active) graphSession.loading = false }
	})
}

export function deleteGraph(location: GraphLocation): Promise<void> {
	return changeGraph(async () => {
		const active = isCurrentGraph(location)
		graphSession.loading = true
		try {
			await flushNow()
			const store = backendStore(location.backendId)
			await store.deleteGraph(location.id)
			if (active) {
				const next = (await store.listGraphs())[0]
				const doc = next ? await store.fetchGraph(next.id) : await store.createGraph(null)
				await restore(location.backendId, doc, await store.fetchView(doc.id))
			}
			backend(location.backendId).revision++
		} finally { graphSession.loading = false }
	})
}

export function detachBackend(id: string): Promise<void> {
	return changeGraph(async () => {
		if (id === LOCAL_BACKEND_ID) throw new Error('本地后端不能移除')
		if (graphSession.backendId === id) {
			graphSession.loading = true
			try {
				await flushNow()
				const store = backendStore(LOCAL_BACKEND_ID)
				const first = (await store.listGraphs())[0]
				const doc = first ? await store.fetchGraph(first.id) : await store.createGraph(null)
				await restore(LOCAL_BACKEND_ID, doc, await store.fetchView(doc.id))
			} finally { graphSession.loading = false }
		}
		removeBackend(id)
	})
}

export function scheduleSave(): void { if (!restoring && graphSession.id) saves.schedule('graph', 400) }
export function scheduleViewSave(): void { if (!restoring && graphSession.id) saves.schedule('view', 600) }
