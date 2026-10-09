// .astelier is a ZIP of one graph directory, portable between workspace backends.
import { unzipSync, zipSync } from 'fflate'
import type { WorkspaceStore } from './store'
import type { DocNode, GraphDocWithId, ViewDoc } from './types'
import { safeStoreFile } from './opfs/paths'

const encodeJson = (value: unknown) => new TextEncoder().encode(JSON.stringify(value, null, 2))
const decodeJson = (bytes: Uint8Array): unknown => JSON.parse(new TextDecoder().decode(bytes))
const object = (value: unknown): value is Record<string, unknown> => !!value && typeof value === 'object' && !Array.isArray(value)
const finite = (value: unknown): value is number => typeof value === 'number' && Number.isFinite(value)
const nodeTypes = new Set(['model', 'prompt', 'image', 'generate', 'preview'])

export interface GraphArchive {
	graph: GraphDocWithId
	view: ViewDoc
	files: Map<string, Uint8Array<ArrayBuffer>>
}

/** Validate all entries before creating anything in the destination. */
export function parseGraphArchive(bytes: Uint8Array): GraphArchive {
	let entries: Record<string, Uint8Array<ArrayBuffer>>
	try { entries = unzipSync(bytes) as Record<string, Uint8Array<ArrayBuffer>> }
	catch { throw new Error('无法读取 .astelier 文件：不是有效的 ZIP 包') }
	const files = new Map<string, Uint8Array<ArrayBuffer>>()
	for (const [path, content] of Object.entries(entries)) {
		if (path === 'graph.json' || path === 'view.json' || path === 'store/') continue
		const name = path.startsWith('store/') ? path.slice(6) : ''
		if (!name || /[/\\:*?"<>|]/.test(name) || !safeStoreFile(name)) throw new Error(`图包包含不支持的路径：${path}`)
		if (!content.length) throw new Error(`图包包含空参考图：${name}`)
		files.set(name, content)
	}
	let graph: unknown, view: unknown
	try { graph = entries['graph.json'] ? decodeJson(entries['graph.json']) : null; view = entries['view.json'] ? decodeJson(entries['view.json']) : { positions: {} } }
	catch { throw new Error('图包中的 JSON 无法解析') }
	if (!object(graph) || (graph.version !== undefined && graph.version !== 1) || typeof graph.id !== 'string' || !graph.id || !Array.isArray(graph.nodes) || !Array.isArray(graph.edges)) throw new Error('图包缺少有效的 graph.json')
	const ids = new Set<string>()
	for (const node of graph.nodes) {
		if (!object(node) || typeof node.id !== 'string' || !node.id || ids.has(node.id) || !nodeTypes.has(String(node.type)) || !object(node.params)) throw new Error('图包包含无效或重复的节点')
		ids.add(node.id)
		if (node.type === 'image') {
			if (node.params.images !== undefined && !Array.isArray(node.params.images)) throw new Error('图包中的参考图列表无效')
			for (const image of (node.params.images ?? []) as unknown[]) {
				if (!object(image) || typeof image.file !== 'string' || !files.has(image.file) || image.dataUrl) throw new Error('图包缺少节点引用的参考图')
			}
		}
	}
	const edgeIds = new Set<string>()
	for (const edge of graph.edges) {
		if (!object(edge) || typeof edge.id !== 'string' || !edge.id || edgeIds.has(edge.id) || typeof edge.source !== 'string' || typeof edge.target !== 'string' || !ids.has(edge.source) || !ids.has(edge.target) || typeof edge.sourcePort !== 'string' || typeof edge.targetPort !== 'string') throw new Error('图包包含无效的连线')
		edgeIds.add(edge.id)
	}
	if (!object(view) || !object(view.positions)) throw new Error('图包中的布局无效')
	const positions: ViewDoc['positions'] = Object.create(null)
	for (const [id, position] of Object.entries(view.positions)) {
		if (!object(position) || !finite(position.x) || !finite(position.y)) throw new Error('图包中的节点位置无效')
		if (ids.has(id)) positions[id] = { x: position.x, y: position.y }
	}
	const cleanView: ViewDoc = { version: 1, positions }
	if (view.viewport !== undefined) {
		const viewport = view.viewport
		if (!object(viewport) || !finite(viewport.x) || !finite(viewport.y) || !finite(viewport.zoom) || viewport.zoom <= 0) throw new Error('图包中的视口无效')
		cleanView.viewport = { x: viewport.x, y: viewport.y, zoom: viewport.zoom }
	}
	return { graph: { ...(graph as unknown as GraphDocWithId), version: 1 }, view: cleanView, files }
}

export async function createGraphArchive(store: WorkspaceStore, id: string): Promise<File> {
	const [graph, view, assets] = await Promise.all([store.fetchGraph(id), store.fetchView(id), store.listGraphStoreFiles(id)])
	const entries: Record<string, Uint8Array> = { 'graph.json': encodeJson(graph), 'view.json': encodeJson(view), 'store/': new Uint8Array() }
	for (const asset of assets) {
		const url = await store.graphStoreUrl(id, asset.name)
		if (!url) throw new Error(`参考图不存在：${asset.name}`)
		const response = await fetch(url)
		if (!response.ok) throw new Error(`读取参考图失败：${asset.name}（HTTP ${response.status}）`)
		entries[`store/${asset.name}`] = new Uint8Array(await response.arrayBuffer())
	}
	const zipped = zipSync(entries)
	// Also catch incomplete exports (e.g. an asset deleted while collecting the graph).
	parseGraphArchive(zipped)
	const title = (graph.title ?? '未命名图').replace(/[/\\:*?"<>|\x00-\x1f]/g, '-').trim().slice(0, 120) || '未命名图'
	return new File([zipped], `${title}.astelier`, { type: 'application/zip' })
}

/** Imports always create a new graph; partial writes are removed on failure. */
export async function importGraphArchive(store: WorkspaceStore, file: File, groupId: string | null = null): Promise<GraphDocWithId> {
	const archive = parseGraphArchive(new Uint8Array(await file.arrayBuffer()))
	const created = await store.createGraph(groupId, archive.graph.title ?? file.name.replace(/\.astelier$/i, ''))
	try {
		const names = new Map<string, string>()
		for (const [name, bytes] of archive.files) {
			const uploaded = await store.uploadGraphStoreFile(created.id, name, new Blob([bytes]))
			names.set(name, uploaded.name)
		}
		const nodes: DocNode[] = archive.graph.nodes.map((node) => node.type === 'image' ? {
			...node,
			params: { ...node.params, images: ((node.params.images ?? []) as { file: string }[]).map((image) => ({ ...image, file: names.get(image.file)! })) },
		} : node)
		await store.putView(created.id, archive.view)
		await store.putGraph(created.id, { version: 1, nodes, edges: archive.graph.edges })
		return { ...created, nodes, edges: archive.graph.edges }
	} catch (error) {
		await store.deleteGraph(created.id).catch(() => {})
		throw error
	}
}
