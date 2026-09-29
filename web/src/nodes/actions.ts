// 节点相关的小动作：删除级联、状态编辑、socket 位置刷新。
//
// 关键约定 1：**绝不要写 $props() 收到的 data 代理**。Svelte 5 中向 props
// 代理写入会把该 prop 标记为“组件自有”，此后 rete 插件经 svelte:component
// 推送的 prop 更新会被跳过，组件再也不重渲染（选中态/参数 UI 僵死）。
// 写入一律经 editNode() 落到 editor 里的原始节点实例上。
//
// 关键约定 2：节点高度变化（展开「更多参数」等 CSS 驱动的高度变化不会
// 触发 rete 的 resize 信号）后，socket 的视觉位置变了但位置缓存过期，
// 连线端点会钉在旧位置脱节。需要 refreshNodeSockets() 主动重发 socket
// 注册信号，让 DOMSocketPosition 重算、位置监听者（连线）跟上。
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

/**
 * 重发节点全部 socket 的 render 信号：DOMSocketPosition 收到 'render' 会
 * 重新计算 socket 中心并通知位置监听者（连线端点）。用于任何改变节点高度
 * 的 UI 变化（展开/折叠面板），rete 自身不发这类 resize 信号。
 */
export function refreshNodeSockets(nodeId: string): void {
	const node = rt.editor?.getNode(nodeId)
	const area = rt.area
	const root = document.querySelector(`.ui-node[data-node-id="${nodeId}"]`)
	if (!area || !node || !root) return
	for (const socketEl of root.querySelectorAll<HTMLElement>('.ui-socket')) {
		const row = socketEl.closest<HTMLElement>('.port-row')
		const side = row?.classList.contains('output') ? 'output' : 'input'
		const key = socketEl.dataset.portKey
		if (!key) continue
		const io = (side === 'output' ? node.outputs : node.inputs) as Record<
			string,
			{ socket: unknown } | undefined
		>
		const socket = io[key]?.socket
		if (!socket) continue
		void area.emit({
			type: 'render',
			data: {
				type: 'socket',
				side,
				key,
				nodeId,
				element: socketEl,
				payload: socket,
			},
		} as Parameters<typeof area.emit>[0])
	}
}
