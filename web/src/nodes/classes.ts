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
	/** 已设置的参数（统一键 → 值）；缺失键 = 默认（不随请求发送） */
	params: Record<string, string | number> = {}
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

export type NodeType = 'model' | 'prompt' | 'image' | 'generate' | 'preview'

/** 节点种类 → 工厂（文档 type 字符串与类一一对应） */
export const factoriesByType: Record<NodeType, () => NodeTypes> = {
	model: () => new ModelNode(),
	prompt: () => new PromptNode(),
	image: () => new LoadImageNode(),
	generate: () => new GenerateNode(),
	preview: () => new PreviewNode(),
}

export function typeOf(node: NodeTypes): NodeType {
	if (node instanceof ModelNode) return 'model'
	if (node instanceof PromptNode) return 'prompt'
	if (node instanceof LoadImageNode) return 'image'
	if (node instanceof GenerateNode) return 'generate'
	return 'preview'
}

/** 业务参数（影响执行结果的结构信息，存文档；不含 UI/运行时状态） */
export function nodeParams(node: NodeTypes): Record<string, string | number> {
	if (node instanceof ModelNode)
		return { provider: node.provider, modelId: node.modelId }
	if (node instanceof PromptNode) return { text: node.text }
	if (node instanceof LoadImageNode)
		return { assetUrl: node.assetUrl ?? '', fileName: node.fileName }
	if (node instanceof GenerateNode) return { ...node.params }
	return {}
}

export function applyParams(node: NodeTypes, params: Record<string, unknown>) {
	if (node instanceof ModelNode) {
		node.provider = String(params.provider ?? '')
		node.modelId = String(params.modelId ?? '')
	} else if (node instanceof PromptNode) {
		node.text = String(params.text ?? '')
	} else if (node instanceof LoadImageNode) {
		node.assetUrl = params.assetUrl ? String(params.assetUrl) : null
		node.fileName = String(params.fileName ?? '')
	} else if (node instanceof GenerateNode) {
		node.params = {}
		for (const [k, v] of Object.entries(params)) {
			if (v === '' || v == null) continue
			node.params[k] = typeof v === 'number' ? v : String(v)
		}
	}
}

/** 最近产物缓存（表现信息）：generate 结果 / preview 展示 */
export function outputOf(node: NodeTypes): string | null {
	if (node instanceof GenerateNode) return node.resultUrl
	if (node instanceof PreviewNode) return node.displayUrl
	return null
}

export function applyOutput(node: NodeTypes, url: string | null) {
	if (node instanceof GenerateNode) node.resultUrl = url
	else if (node instanceof PreviewNode) node.displayUrl = url
}
