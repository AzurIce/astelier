// 全图求值：消费结构文档快照（UI 无关），执行中的加载/错误/产物
// 状态回写到对应的存活节点实例上。
import { apiGenerate } from './api'
import { toDoc, type GraphDoc } from './graphDoc'
import { rt, runningNodes } from './runtime'
import { scheduleViewSave } from './graphStore'

type Outputs = Record<string, unknown>

/** 全图执行；返回完成的 Generate 节点数。失败抛 Error（错误同时落在节点上）。 */
export async function runPipeline(): Promise<number> {
	const editor = rt.editor!
	const area = rt.area!
	if (!editor || !area) throw new Error('画布未就绪')

	const doc: GraphDoc = toDoc()
	const cache = new Map<string, Outputs>()
	const running = new Set<string>()
	let generated = 0

	async function evalNode(id: string): Promise<Outputs> {
		if (cache.has(id)) return cache.get(id)!
		if (running.has(id)) throw new Error('图中存在环')
		running.add(id)
		const node = doc.nodes.find((n) => n.id === id)
		if (!node) throw new Error(`节点不存在：${id}`)

		const inputs: Record<string, unknown> = {}
		for (const c of doc.edges) {
			if (c.target !== id) continue
			const src = await evalNode(c.source)
			const value = src[c.sourcePort]
			if (value === undefined) continue
			if (c.targetPort in inputs) {
				// 多条连线汇入同一输入 → 数组（如多条 prompt）
				const prev = inputs[c.targetPort]
				inputs[c.targetPort] = Array.isArray(prev) ? [...prev, value] : [prev, value]
			} else {
				inputs[c.targetPort] = value
			}
		}

		const outputs = await runNode(node, inputs)
		running.delete(id)
		cache.set(id, outputs)
		return outputs
	}

	for (const node of doc.nodes) {
		await evalNode(node.id)
		if (node.type === 'generate') generated++
	}
	return generated
}

function joinValues(v: unknown): string | undefined {
	if (v === undefined || v === null) return undefined
	const arr = Array.isArray(v) ? v : [v]
	const texts = arr.map(String).filter((s) => s.length > 0)
	return texts.length > 0 ? texts.join(', ') : undefined
}

async function runNode(
	node: GraphDoc['nodes'][number],
	inputs: Record<string, unknown>,
): Promise<Outputs> {
	const area = rt.area!
	// 运行时状态（busy/error/产物）回写存活实例；计算只依赖文档参数
	const live = rt.editor?.getNode(node.id)

	if (node.type === 'model') {
		const provider = String(node.params.provider ?? '')
		const modelId = String(node.params.modelId ?? '')
		if (!provider || !modelId) throw new Error('Model 节点未选择模型')
		return { model: `${provider}:${modelId}` }
	}
	if (node.type === 'prompt') {
		return { text: String(node.params.text ?? '') }
	}
	if (node.type === 'image') {
		const url = String(node.params.assetUrl ?? '')
		if (!url) throw new Error('Image 节点未上传图片')
		return { image: url }
	}
	if (node.type === 'preview') {
		const img = Array.isArray(inputs.image) ? inputs.image[0] : inputs.image
		const { PreviewNode } = await import('./nodes/classes')
		if (live instanceof PreviewNode) {
			live.displayUrl = typeof img === 'string' ? img : null
			area.update('node', live.id)
			scheduleViewSave()
		}
		return {}
	}
	if (node.type === 'generate') {
		const { GenerateNode } = await import('./nodes/classes')
		const model = inputs.model
		if (typeof model !== 'string') throw new Error('Generate 未连接 model')
		const prompt = joinValues(inputs.prompt)
		if (!prompt) throw new Error('Generate 未连接 prompt（或 prompt 为空）')
		const refs = (Array.isArray(inputs.image) ? inputs.image : [inputs.image]).filter(
			(v): v is string => typeof v === 'string',
		)

		const gen = live instanceof GenerateNode ? live : null
		if (gen) {
			gen.busy = true
			gen.error = null
			area.update('node', gen.id)
		}
		runningNodes.add(node.id)
		try {
			const result = await apiGenerate({
				model,
				prompt,
				params: node.params,
				imageUrls: refs,
			})
			if (gen) {
				gen.resultUrl = result.imageUrl
				area.update('node', gen.id)
				scheduleViewSave()
			}
		} catch (e) {
			const msg = e instanceof Error ? e.message : String(e)
			if (gen) {
				gen.error = msg
				area.update('node', gen.id)
			}
			throw new Error(msg)
		} finally {
			if (gen) {
				gen.busy = false
				area.update('node', gen.id)
			}
			runningNodes.delete(node.id)
		}
		return { image: gen?.resultUrl! }
	}
	return {}
}
