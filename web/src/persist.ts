// 画布持久化（localStorage 首版；后续接服务端 /api/graphs）
import {
	applyNodeData,
	nodeData,
	nodeFactories,
	type NodeTypes,
} from './nodes/classes'
import { connect } from './nodes/types'
import { rt } from './runtime'

const KEY = 'atelier-graph-v1'

interface SavedGraph {
	nodes: {
		label: string
		x: number
		y: number
		data: Record<string, unknown>
	}[]
	connections: { source: number; output: string; target: number; input: string }[]
}

let timer: ReturnType<typeof setTimeout> | null = null

export function scheduleSave() {
	if (timer) clearTimeout(timer)
	timer = setTimeout(saveGraph, 400)
}

export function saveGraph() {
	const editor = rt.editor
	const area = rt.area
	if (!editor || !area) return
	const saved: SavedGraph = { nodes: [], connections: [] }
	const indexById = new Map<string, number>()

	for (const node of editor.getNodes()) {
		const view = area.nodeViews.get(node.id)
		indexById.set(node.id, saved.nodes.length)
		saved.nodes.push({
			label: node.label,
			x: view?.position.x ?? 0,
			y: view?.position.y ?? 0,
			data: nodeData(node),
		})
	}
	for (const conn of editor.getConnections()) {
		const src = indexById.get(String((conn as unknown as Record<string, unknown>).source))
		const dst = indexById.get(String((conn as unknown as Record<string, unknown>).target))
		if (src === undefined || dst === undefined) continue
		const output = String(
			(conn as unknown as Record<string, unknown>).output ??
				(conn as unknown as Record<string, unknown>).sourceOutput,
		)
		const input = String(
			(conn as unknown as Record<string, unknown>).input ??
				(conn as unknown as Record<string, unknown>).targetInput,
		)
		saved.connections.push({ source: src, output, target: dst, input })
	}
	try {
		localStorage.setItem(KEY, JSON.stringify(saved))
	} catch {
		/* 存储满时静默放弃 */
	}
}

/** 恢复画布；返回是否恢复了内容 */
export async function restoreGraph(): Promise<boolean> {
	const editor = rt.editor
	const area = rt.area
	if (!editor || !area) return false
	let saved: SavedGraph | null = null
	try {
		const raw = localStorage.getItem(KEY)
		saved = raw ? (JSON.parse(raw) as SavedGraph) : null
	} catch {
		saved = null
	}
	if (!saved || saved.nodes.length === 0) return false

	const created: NodeTypes[] = []
	for (const n of saved.nodes) {
		const factory = nodeFactories[n.label]
		if (!factory) continue
		const node = factory()
		applyNodeData(node, n.data)
		await editor.addNode(node)
		await area.translate(node.id, { x: n.x, y: n.y })
		created.push(node)
	}
	for (const c of saved.connections) {
		const src = created[c.source]
		const dst = created[c.target]
		if (!src || !dst) continue
		if (!src.outputs[c.output] || !dst.inputs[c.input]) continue
		await editor.addConnection(connect(src, c.output, dst, c.input))
	}
	return true
}
