import { ClassicPreset } from 'rete'
import type { ImageRef } from '../../images/types'
import type { NodeType } from '../../workspace/types'

// 端口类型：socket 实例即类型（连线两端 socket 相同才兼容）
export const sockets = {
	model: new ClassicPreset.Socket('model'),
	text: new ClassicPreset.Socket('text'),
	image: new ClassicPreset.Socket('image'),
}

export class ModelNode extends ClassicPreset.Node {
	static type = 'model' as const
	provider = $state('')
	modelId = $state('')
	constructor() {
		super('Model')
		this.addOutput('model', new ClassicPreset.Output(sockets.model))
	}
}

export class PromptNode extends ClassicPreset.Node {
	static type = 'prompt' as const
	text = $state('')
	constructor() {
		super('Prompt')
		this.addOutput('text', new ClassicPreset.Output(sockets.text))
	}
}

export class LoadImageNode extends ClassicPreset.Node {
	static type = 'image' as const
	/** 多张参考图；顺序即发送给上游的 image[] 顺序 */
	images = $state<ImageRef[]>([])
	constructor() {
		super('Image')
		this.addOutput('image', new ClassicPreset.Output(sockets.image))
	}

	/** 追加（引用 / 内容去重），返回是否真的加进去了 */
	addImage(ref: ImageRef): boolean {
		if (!ref.file) return false
		const existing = this.images.find((i) => i.file === ref.file || (ref.hash && i.hash === ref.hash))
		if (existing) {
			// 老文档没有指纹；首次重新导入同一引用时补齐，后续也能按内容去重。
			if (ref.hash && !existing.hash) this.images = this.images.map((i) => i === existing ? { ...i, hash: ref.hash } : i)
			return false
		}
		this.images = [...this.images, ref]
		return true
	}

	removeImage(file: string): void {
		this.images = this.images.filter((i) => i.file !== file)
	}

	/** 拖拽和键盘共用顺序更新入口。 */
	moveImage(file: string, index: number): void {
		const from = this.images.findIndex((image) => image.file === file)
		if (from < 0) return
		const images = [...this.images]
		const [image] = images.splice(from, 1)
		images.splice(Math.max(0, Math.min(index, images.length)), 0, image)
		this.images = images
	}
}

export class GenerateNode extends ClassicPreset.Node {
	static type = 'generate' as const
	/** 已设置的参数（统一键 → 值）；恢复文档时补齐协议默认值。 */
	params = $state<Record<string, string | number | boolean | object | null>>({})
	resultUrls = $state<string[]>([])
	busy = $state(false)
	error = $state<string | null>(null)
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
	displayUrl = $state<string | null>(null)
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

