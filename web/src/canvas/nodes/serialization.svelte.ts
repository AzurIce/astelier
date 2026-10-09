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
		node.providerBackendId = String(params.providerBackendId ?? '')
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
 */
export function imageRefsOf(node: unknown): string[] {
	const n = node as { images?: unknown } | null | undefined
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
	return out
}
