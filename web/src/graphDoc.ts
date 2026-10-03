// editor（rete 实例）↔ 结构/表现文档 的唯一转换点。
// 结构文档 UI 无关：type/params/端口级连线；执行引擎消费文档快照。
import type { NodeTypes, NodeType } from './nodes/classes'
import { applyParams, applyOutput, factoriesByType, nodeParams, outputOf, typeOf } from './nodes/classes'
import { connKeys } from './nodes/conn'
import { connect } from './nodes/types'
import { rt } from './runtime'

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
	outputs: Record<string, string>
}

/** 编辑器当前状态 → 结构文档 */
export function toDoc(): GraphDoc {
	const editor = rt.editor!
	const nodes: DocNode[] = editor.getNodes().map((n) => ({
		id: n.id,
		type: typeOf(n),
		params: nodeParams(n),
	}))
	const edges: DocEdge[] = editor.getConnections().map((c) => {
		const k = connKeys(c as unknown as Record<string, unknown>)
		return {
			id: `${k.source}:${k.output}->${k.target}:${k.input}`,
			source: k.source,
			sourcePort: k.output,
			target: k.target,
			targetPort: k.input,
		}
	})
	return { version: 1, nodes, edges }
}

/** 表现文档：位置 / 视口 / 最近产物 */
export function toViewDoc(): ViewDoc {
	const editor = rt.editor!
	const area = rt.area!
	const t = area.area.transform
	const positions: Record<string, { x: number; y: number }> = {}
	const outputs: Record<string, string> = {}
	for (const n of editor.getNodes()) {
		const view = area.nodeViews.get(n.id)
		positions[n.id] = view
			? { x: view.position.x, y: view.position.y }
			: { x: 0, y: 0 }
		const out = outputOf(n)
		if (out) outputs[n.id] = out
	}
	return {
		version: 1,
		positions,
		viewport: { x: t.x, y: t.y, zoom: t.k },
		outputs,
	}
}

/** 文档 → 编辑器。保留文档节点 id，使 view/outputs/runs 关联跨会话稳定。 */
export async function loadDoc(doc: GraphDoc, view?: ViewDoc): Promise<void> {
	const editor = rt.editor!
	const area = rt.area!

	// 视口先行（holder 的 transform 需手动刷新）
	if (view?.viewport) {
		const t = area.area.transform
		t.x = view.viewport.x
		t.y = view.viewport.y
		t.k = view.viewport.zoom > 0 ? view.viewport.zoom : 1
		area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${t.k})`
	}

	const created: NodeTypes[] = []
	for (const dn of doc.nodes) {
		const factory = factoriesByType[dn.type]
		if (!factory) continue
		const node = factory()
		node.id = dn.id // id 为公开字段，addNode 前覆盖即保留文档 id
		applyParams(node, dn.params ?? {})
		await editor.addNode(node)
		const pos = view?.positions?.[dn.id] ?? defaultPosition(dn.type, created.length)
		await area.translate(node.id, pos)
		created.push(node)
	}
	for (const de of doc.edges) {
		const src = editor.getNode(de.source)
		const dst = editor.getNode(de.target)
		if (!src || !dst) continue
		if (!src.outputs[de.sourcePort] || !dst.inputs[de.targetPort]) continue
		await editor.addConnection(connect(src, de.sourcePort, dst, de.targetPort))
	}
	// 最近产物回填
	if (view?.outputs) {
		for (const [id, url] of Object.entries(view.outputs)) {
			const node = editor.getNode(id)
			if (node && url) applyOutput(node, url)
		}
	}
}

function defaultPosition(type: NodeType, index: number): { x: number; y: number } {
	const col: Record<NodeType, number> = {
		model: 60,
		prompt: 60,
		image: 60,
		generate: 420,
		preview: 780,
	}
	return { x: col[type] ?? 60, y: 120 + index * 180 }
}
