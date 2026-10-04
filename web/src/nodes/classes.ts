import { ClassicPreset } from 'rete'
import { OPENAI_IMAGE_PARAMS } from '../apiParams'

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

/** 节点引用的一张图：图内 store 文件名（内联感知：UI 只认「这张图」） */
export interface ImageRef {
	/** 图内 store 文件名（即引用） */
	file: string
	/** 展示名（默认同 file） */
	name: string
	/** 展示用尺寸（store 引用时来自 manifest） */
	w?: number
	h?: number
	/** 内容指纹：不同文件名的相同图片也只保留一份。 */
	hash?: string
}

export class LoadImageNode extends ClassicPreset.Node {
	static type = 'image' as const
	/** 多张参考图；顺序即发送给上游的 image[] 顺序 */
	images: ImageRef[] = []
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
	/** 已设置的参数（统一键 → 值）；缺失键 = 默认（不随请求发送） */
	params: Record<string, string | number | boolean | object | null> = {}
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
export function nodeParams(node: NodeTypes): Record<string, string | number | boolean | object | null> {
	if (node instanceof ModelNode)
		return { provider: node.provider, modelId: node.modelId }
	if (node instanceof PromptNode) return { text: node.text }
	if (node instanceof LoadImageNode) return { images: node.images.map((i) => ({ ...i })) }
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
		const list: ImageRef[] = []
		const push = (file: string, name?: string, w?: number, h?: number, hash?: string) => {
			const f = file?.trim()
			if (!f || list.some((i) => i.file === f)) return
			list.push({
				file: f,
				name: (name && name.trim()) || f,
				...(w != null ? { w } : {}),
				...(h != null ? { h } : {}),
				...(hash ? { hash } : {}),
			})
		}
		for (const it of Array.isArray(params.images) ? params.images : []) {
			if (!it || typeof it !== 'object') continue
			const r = it as Record<string, unknown>
			push(
				String(r.file ?? ''),
				typeof r.name === 'string' ? r.name : undefined,
				typeof r.w === 'number' ? r.w : undefined,
				typeof r.h === 'number' ? r.h : undefined,
				typeof r.hash === 'string' ? r.hash : undefined,
			)
		}
		// 过渡：旧文档存的 fileName / refFile / {store,file} / /gstore/… 引用
		// 一律折叠成 images[0]。data URL（base64 内联）不再支持 → 视为空引用。
		if (!list.length) {
			const ref = params.ref as { file?: unknown } | undefined
			const legacy =
				(typeof params.refFile === 'string' ? params.refFile : '') ||
				(ref && typeof ref.file === 'string' ? decodeURIComponent(ref.file) : '') ||
				(typeof params.assetUrl === 'string' &&
				(params.assetUrl as string).startsWith('/gstore/')
					? decodeURIComponent((params.assetUrl as string).split('/').pop() ?? '')
					: '')
			const name = typeof params.fileName === 'string' ? params.fileName : ''
			push(
				legacy,
				name || legacy,
				typeof params.w === 'number' ? params.w : undefined,
				typeof params.h === 'number' ? params.h : undefined,
			)
		}
		node.images = list
	} else if (node instanceof GenerateNode) {
		node.params = {}
		for (const [k, v] of Object.entries(params)) {
			if (v === '' || v == null) continue
			node.params[k] = typeof v === 'number' ? v : String(v)
		}
		// 始终完整发送：缺失的协议参数按档案默认补齐（老图 / 直连接口兜底，
		// 与后端 launch_run 的归一化同规则）
		for (const p of OPENAI_IMAGE_PARAMS) {
			if (p.key in node.params) continue
			node.params[p.key] = typeof p.def === 'number' ? p.def : String(p.def ?? '')
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

/**
 * 读任意节点的图片引用（不 import 具体类，供回收清理 / 执行引擎用）。
 * 旧实例（LoadImageNode 的单一 refFile）也兼容。
 */
export function imageRefsOf(node: unknown): string[] {
	const n = node as { images?: unknown; refFile?: unknown } | null | undefined
	if (!n) return []
	const out: string[] = []
	if (Array.isArray(n.images)) {
		for (const i of n.images) {
			const file = (i as { file?: unknown })?.file
			if (typeof file === 'string' && file) out.push(file)
		}
		return out
	}
	if (typeof n.refFile === 'string' && n.refFile) out.push(n.refFile)
	return out
}
