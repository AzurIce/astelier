// 每次执行使用文档快照；运行状态和产物属于原节点，不归档。
import { imageGenerator } from '../generation/generator'
import { toDoc } from './document'
import { rt, runningNodes } from './runtime'
import { activeGraphId, graphSession } from './session.svelte'
import { graphStoreUrl } from '../workspace/imageFiles'
import { GenerateNode, LoadImageNode, PreviewNode } from './nodes/model.svelte'

type Outputs = Record<string, unknown>

/** 全图执行；返回完成的 Generate 节点数。 */
export async function runPipeline(): Promise<number> {
	const editor = rt.editor
	const graphId = activeGraphId()
	if (!editor || !rt.area || !graphId) throw new Error('画布未就绪')

	const doc = toDoc()
	const nodes = new Map(editor.getNodes().map((node) => [node.id, node]))
	// 临时参考图不进入文档，仍作为本次执行的输入快照；
	// 持久引用解析成会话 URL（本地为 blob:，远端为绝对地址）。
	const images = new Map<string, string[]>()
	for (const [id, node] of nodes) {
		if (node instanceof LoadImageNode) images.set(id, await resolveRefs(graphId, (node as LoadImageNode).images))
	}
	const cache = new Map<string, Outputs>()
	const running = new Set<string>()
	let generated = 0

	function isActive(id: string): boolean {
		return !graphSession.loading && rt.editor === editor && activeGraphId() === graphId && editor?.getNode(id) === nodes.get(id)
	}
	function requireActive(id: string): void {
		if (!isActive(id)) throw new Error('原画布已切换或节点已删除，执行已结束')
	}

	async function evalNode(id: string): Promise<Outputs> {
		requireActive(id)
		if (cache.has(id)) return cache.get(id)!
		if (running.has(id)) throw new Error('图中存在环')
		running.add(id)
		const node = doc.nodes.find((node) => node.id === id)
		if (!node) throw new Error(`节点不存在：${id}`)

		const inputs: Record<string, unknown> = {}
		for (const edge of doc.edges) {
			if (edge.target !== id) continue
			const source = await evalNode(edge.source)
			const value = source[edge.sourcePort]
			if (value === undefined) continue
			if (edge.targetPort in inputs) {
				const previous = inputs[edge.targetPort]
				inputs[edge.targetPort] = Array.isArray(previous) ? [...previous, value] : [previous, value]
			} else inputs[edge.targetPort] = value
		}
		requireActive(id)
		const live = nodes.get(id)!
		let outputs: Outputs = {}
		if (node.type === 'model') {
			const provider = String(node.params.provider ?? '')
			const modelId = String(node.params.modelId ?? '')
			if (!provider || !modelId) throw new Error('Model 节点未选择模型')
			outputs = { model: `${provider}:${modelId}` }
		} else if (node.type === 'prompt') {
			outputs = { text: String(node.params.text ?? '') }
		} else if (node.type === 'image') {
			const refs = images.get(id) ?? []
			if (!refs.length) throw new Error('Image 节点未上传图片')
			outputs = { image: refs }
		} else if (live instanceof PreviewNode) {
			live.displayUrl = collectStrings(inputs.image)[0] ?? null
		} else if (live instanceof GenerateNode) {
			const model = inputs.model
			const prompt = joinValues(inputs.prompt)
			if (typeof model !== 'string') throw new Error('Generate 未连接 model')
			if (!prompt) throw new Error('Generate 未连接 prompt（或 prompt 为空）')
			live.busy = true
			live.error = null
			runningNodes.add(id)
			try {
				const result = await imageGenerator().generate({ model, prompt, params: node.params, imageUrls: collectStrings(inputs.image) })
				requireActive(id)
				if (!result.imageUrls?.length) throw new Error('响应中没有图片数据')
				live.resultUrls = result.imageUrls
				outputs = { image: result.imageUrls }
				generated++
			} catch (error) {
				if (isActive(id)) live.error = error instanceof Error ? error.message : String(error)
				throw error
			} finally {
				live.busy = false
				runningNodes.delete(id)
			}
		}
		running.delete(id)
		cache.set(id, outputs)
		return outputs
	}

	for (const node of doc.nodes) await evalNode(node.id)
	return generated
}

async function resolveRefs(graphId: string, refs: { dataUrl?: string; file: string }[]): Promise<string[]> {
	const out: string[] = []
	for (const image of refs) {
		if (image.dataUrl) {
			out.push(image.dataUrl)
			continue
		}
		const url = await graphStoreUrl(graphId, image.file)
		if (!url) throw new Error(`参考图文件不存在：${image.file}`)
		out.push(url)
	}
	return out
}

function collectStrings(value: unknown): string[] {
	if (Array.isArray(value)) return value.flatMap(collectStrings)
	return typeof value === 'string' && value.length > 0 ? [value] : []
}

function joinValues(value: unknown): string | undefined {
	const texts = collectStrings(value)
	return texts.length ? texts.join(', ') : undefined
}
