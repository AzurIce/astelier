// Connection（ rete 实例或纯形状）→ 纯 id/键；及 model 连线解析。
// 独立小模块：editor.ts、exec、节点组件都要用，避免经 editor.ts 成环
// （runtime 只有类型导入，这里引它不会产生运行时循环）。
import { rt } from '../runtime'
import { ModelNode } from './classes'

export function connKeys(c: Record<string, unknown>) {
	const nodeId = (v: unknown) =>
		typeof v === 'string' ? v : String((v as { id?: string })?.id ?? '')
	return {
		source: nodeId(c.source),
		target: nodeId(c.target),
		output: String(c.sourceOutput ?? c.output ?? ''),
		input: String(c.targetInput ?? c.input ?? ''),
	}
}

/** 某节点 model 输入所连的 Model 节点（无连线返回 null） */
export function connectedModelNode(nodeId: string): ModelNode | null {
	const editor = rt.editor
	if (!editor) return null
	for (const c of editor.getConnections()) {
		const k = connKeys(c as unknown as Record<string, unknown>)
		if (k.target !== nodeId || k.input !== 'model') continue
		const src = editor.getNode(k.source)
		if (src instanceof ModelNode) return src
	}
	return null
}
