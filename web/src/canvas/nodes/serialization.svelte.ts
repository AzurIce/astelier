// 节点参数的持久化与恢复，以及持久图片引用的提取。
import { ModelNode, PromptNode, LoadImageNode, GenerateNode, type NodeTypes } from './model.svelte'
import type { ImageRef } from '../../images/types'

/** 业务参数（影响执行结果的结构信息，存文档；不含 UI/运行时状态） */
export function nodeParams(node: NodeTypes): Record<string, string | number | boolean | object | null> {
	if (node instanceof ModelNode)
		return { providerBackendId: node.providerBackendId, provider: node.provider, modelId: node.modelId }
	if (node instanceof PromptNode) return { text: node.text }
	if (node instanceof LoadImageNode) return { images: node.images.filter((i) => !i.dataUrl).map((i) => ({ ...i })) }
	if (node instanceof GenerateNode) return $state.snapshot(node.params)
	return {}
}

export function applyParams(node: NodeTypes, params: Record<string, unknown>) {
	if (node instanceof ModelNode) {
		node.providerBackendId = String(params.providerBackendId ?? 'local')
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

	}
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
			if ((i as { dataUrl?: unknown })?.dataUrl) continue
			const file = (i as { file?: unknown })?.file
			if (typeof file === 'string' && file) out.push(file)
		}
		return out
	}
	if (typeof n.refFile === 'string' && n.refFile) out.push(n.refFile)
	return out
}
