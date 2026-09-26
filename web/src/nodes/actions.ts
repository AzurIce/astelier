// 节点相关的小动作：删除级联、状态编辑等，集中一处避免循环依赖
// （editor.ts 的删除快捷键也走这里）。
//
// 关键约定：**绝不要写 $props() 收到的 data 代理**。Svelte 5 中向 props
// 代理写入会把该 prop 标记为“组件自有”，此后 rete 插件经 svelte:component
// 推送的 prop 更新会被跳过，组件再也不重渲染（选中态/参数 UI 全部僵死）。
// 写入一律经 editNode() 落到 editor 里的原始节点实例上。
import { rt } from '../runtime'
import { connKeys } from './conn'
import { scheduleSave } from '../graphStore'

/** 删除节点并级联清理相邻连线 */
export async function removeNodeCascade(nodeId: string): Promise<void> {
	const editor = rt.editor
	if (!editor) return
	for (const conn of editor.getConnections()) {
		const k = connKeys(conn as unknown as Record<string, unknown>)
		if (k.source === nodeId || k.target === nodeId) {
			await editor.removeConnection(conn.id)
		}
	}
	await editor.removeNode(nodeId)
}

/** 触发节点组件重渲染（rete 的 svelte:component prop 更新路径）+ 节流落盘 */
export function touchNode(nodeId: string): void {
	rt.area?.update('node', nodeId)
	scheduleSave()
}

/** 编辑节点业务状态：改原始实例 → 重渲染 → 保存 */
export function editNode<T>(nodeId: string, fn: (node: T) => void): void {
	const node = rt.editor?.getNode(nodeId) as T | undefined
	if (!node) return
	fn(node)
	touchNode(nodeId)
}
