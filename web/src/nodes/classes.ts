import { ClassicPreset } from 'rete'

// 端口类型：socket 实例即类型（连线两端 socket 相同才兼容）
export const sockets = {
	model: new ClassicPreset.Socket('model'),
	text: new ClassicPreset.Socket('text'),
	image: new ClassicPreset.Socket('image'),
}

export class ModelNode extends ClassicPreset.Node {
	static type = 'model' as const
	provider = ''
	modelId = ''
	constructor() {
		super('Model')
		this.addOutput('model', new ClassicPreset.Output(sockets.model))
	}
}

export class PromptNode extends ClassicPreset.Node {
	static type = 'prompt' as const
	text = ''
	constructor() {
		super('Prompt')
		this.addOutput('text', new ClassicPreset.Output(sockets.text))
	}
}

export class LoadImageNode extends ClassicPreset.Node {
	static type = 'image' as const
	assetUrl: string | null = null
	fileName = ''
	constructor() {
		super('Image')
		this.addOutput('image', new ClassicPreset.Output(sockets.image))
	}
}

export class GenerateNode extends ClassicPreset.Node {
	static type = 'generate' as const
	steps = 20
	cfgScale = 7
	seed = Math.floor(Math.random() * 99999)
	resultUrl: string | null = null
	busy = false
	error: string | null = null
	constructor() {
		super('Generate')
		this.addInput('model', new ClassicPreset.Input(sockets.model, 'model'))
		// 多条 prompt 汇入时按顺序拼接
		this.addInput('prompt', new ClassicPreset.Input(sockets.text, 'prompt', true))
		this.addInput('image', new ClassicPreset.Input(sockets.image, 'ref', true))
		this.addOutput('image', new ClassicPreset.Output(sockets.image))
	}
}

export class PreviewNode extends ClassicPreset.Node {
	static type = 'preview' as const
	displayUrl: string | null = null
	constructor() {
		super('Preview')
		this.addInput('image', new ClassicPreset.Input(sockets.image, 'image'))
	}
}

export type NodeTypes =
	| ModelNode
	| PromptNode
	| LoadImageNode
	| GenerateNode
	| PreviewNode

export const nodeFactories: Record<string, () => NodeTypes> = {
	Model: () => new ModelNode(),
	Prompt: () => new PromptNode(),
	Image: () => new LoadImageNode(),
	Generate: () => new GenerateNode(),
	Preview: () => new PreviewNode(),
}

/** 节点自定义字段的序列化 / 反序列化（画布持久化用） */
export function nodeData(node: ClassicPreset.Node): Record<string, unknown> {
	if (node instanceof ModelNode)
		return { provider: node.provider, modelId: node.modelId }
	if (node instanceof PromptNode) return { text: node.text }
	if (node instanceof LoadImageNode)
		return { assetUrl: node.assetUrl, fileName: node.fileName }
	if (node instanceof GenerateNode)
		return {
			steps: node.steps,
			cfgScale: node.cfgScale,
			seed: node.seed,
			resultUrl: node.resultUrl,
		}
	return {}
}

export function applyNodeData(node: ClassicPreset.Node, data: Record<string, unknown>) {
	if (node instanceof ModelNode) {
		node.provider = String(data.provider ?? '')
		node.modelId = String(data.modelId ?? '')
	} else if (node instanceof PromptNode) {
		node.text = String(data.text ?? '')
	} else if (node instanceof LoadImageNode) {
		node.assetUrl = data.assetUrl ? String(data.assetUrl) : null
		node.fileName = String(data.fileName ?? '')
	} else if (node instanceof GenerateNode) {
		node.steps = Number(data.steps ?? 20)
		node.cfgScale = Number(data.cfgScale ?? 7)
		node.seed = Number(data.seed ?? 0)
		node.resultUrl = data.resultUrl ? String(data.resultUrl) : null
	}
}
