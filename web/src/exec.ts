// 全图求值：按连接拓扑递归执行节点，输出缓存于本次 run。
import { apiGenerate } from './api'
import {
	GenerateNode,
	LoadImageNode,
	ModelNode,
	PreviewNode,
	PromptNode,
} from './nodes/classes'
import { connKeys } from './editor'
import { rt } from './runtime'

type Outputs = Record<string, unknown>

/** 全图执行；返回完成的 Generate 节点数。失败抛 Error（错误同时落在节点上）。 */
export async function runPipeline(): Promise<number> {
	const editor = rt.editor!
	const area = rt.area!
	if (!editor || !area) throw new Error('画布未就绪')

	const conns = editor.getConnections().map((c) =>
		connKeys(c as unknown as Record<string, unknown>),
	)
	const cache = new Map<string, Outputs>()
	const running = new Set<string>()
	let generated = 0
	async function evalNode(id: string): Promise<Outputs> {
		if (cache.has(id)) return cache.get(id)!
		if (running.has(id)) throw new Error('图中存在环')
		running.add(id)
		const node = editor.getNode(id)
		if (!node) throw new Error(`节点不存在：${id}`)

		const inputs: Record<string, unknown> = {}
		for (const c of conns) {
			if (c.target !== id) continue
			const src = await evalNode(c.source)
			const value = src[c.output]
			if (value === undefined) continue
			if (c.input in inputs) {
				// 多条连线汇入同一输入 → 数组（如多条 prompt）
				const prev = inputs[c.input]
				inputs[c.input] = Array.isArray(prev) ? [...prev, value] : [prev, value]
			} else {
				inputs[c.input] = value
			}
		}

		const outputs = await runNode(node, inputs)
		running.delete(id)
		cache.set(id, outputs)
		return outputs
	}

	for (const node of editor.getNodes()) {
		await evalNode(node.id)
		if (node instanceof GenerateNode) generated++
	}
	return generated
}

function joinValues(v: unknown): string | undefined {
	if (v === undefined || v === null) return undefined
	const arr = Array.isArray(v) ? v : [v]
	const texts = arr.map(String).filter((s) => s.length > 0)
	return texts.length > 0 ? texts.join(', ') : undefined
}

async function runNode(node: unknown, inputs: Record<string, unknown>): Promise<Outputs> {
	const area = rt.area!

	if (node instanceof ModelNode) {
		if (!node.provider || !node.modelId) throw new Error('Model 节点未选择模型')
		return { model: `${node.provider}:${node.modelId}` }
	}
	if (node instanceof PromptNode) {
		return { text: node.text }
	}
	if (node instanceof LoadImageNode) {
		if (!node.assetUrl) throw new Error('Image 节点未上传图片')
		return { image: node.assetUrl }
	}
	if (node instanceof PreviewNode) {
		const img = Array.isArray(inputs.image) ? inputs.image[0] : inputs.image
		node.displayUrl = typeof img === 'string' ? img : null
		area.update('node', node.id)
		return {}
	}
	if (node instanceof GenerateNode) {
		const model = inputs.model
		if (typeof model !== 'string') throw new Error('Generate 未连接 model')
		const prompt = joinValues(inputs.prompt)
		if (!prompt) throw new Error('Generate 未连接 prompt（或 prompt 为空）')
		const refs = (Array.isArray(inputs.image) ? inputs.image : [inputs.image]).filter(
			(v): v is string => typeof v === 'string',
		)
		// /api/generate 目前单参考图；多图走 /api/runs（后续）
		const referenceImageUrl = refs.length > 0 ? refs[refs.length - 1] : undefined

		node.busy = true
		node.error = null
		area.update('node', node.id)
		try {
			const result = await apiGenerate({
				model,
				prompt,
				steps: node.steps,
				cfgScale: node.cfgScale,
				seed: node.seed,
				referenceImageUrl,
			})
			node.resultUrl = result.imageUrl
		} catch (e) {
			node.error = e instanceof Error ? e.message : String(e)
			area.update('node', node.id)
			throw e
		} finally {
			node.busy = false
			area.update('node', node.id)
		}
		return { image: node.resultUrl! }
	}
	return {}
}
