// 节点相关的小动作：删除级联、状态编辑、socket 位置刷新。
//
// 业务修改统一经 editNode 写入响应式节点，并安排文档保存。
// CSS 高度变化不属于 Rete resize 信号，由 editor 的 ResizeObserver
// 统一调用 refreshNodeSockets 更新连线端点坐标。
import { rt } from '../runtime'
import { connKeys } from './conn'
import { imageRefsOf } from './classes.svelte'
import { scheduleSave, activeGraphId } from '../graphStore.svelte'

/**
 * 删除节点并级联清理相邻连线。
 * LoadImage 节点额外回收图内 store 文件：先记下被删节点引用的文件，删完
 * 再扫其余节点的现役引用，只删「图内已无任何引用」的那些——删一个 LoadImage
 * 不会误伤被其他节点共享的文件。
 */
export async function removeNodeCascade(nodeId: string): Promise<void> {
	const editor = rt.editor
	if (!editor) return
	// 删除前抓被删节点的图内引用（removeNode 后实例就被销毁了）
	const target = editor.getNode(nodeId)
	const orphanRefs = new Set<string>(imageRefsOf(target))

	for (const conn of editor.getConnections()) {
		const k = connKeys(conn as unknown as Record<string, unknown>)
		if (k.source === nodeId || k.target === nodeId) {
			await editor.removeConnection(conn.id)
		}
	}
	await editor.removeNode(nodeId)

	if (orphanRefs.size) await cleanupGraphStoreRefs(orphanRefs)
}

/**
 * 解除引用后的文件回收：调用方必须已经改完引用（组件先 editNode 再调这里），
 * 本函数扫全图现役引用，只删「已无任何节点引用」的文件。
 * 用于节点内单张移除 / 清空全部——以前只解引用，文件永久残留在图 store。
 */
export async function releaseGraphStoreFiles(files: string[]): Promise<void> {
	if (files.length) await cleanupGraphStoreRefs(new Set(files))
}

/** 删除一组图内引用文件（跳过仍被其他节点引用的） */
async function cleanupGraphStoreRefs(refs: Set<string>): Promise<void> {
	const gid = activeGraphId()
	const editor = rt.editor
	if (!gid || !editor) return
	// 图内其余节点的现役引用
	for (const n of editor.getNodes()) {
		for (const f of imageRefsOf(n)) refs.delete(f)
	}
	for (const name of refs) {
		try {
			await fetch(
				`/api/graphs/${encodeURIComponent(gid)}/store/${encodeURIComponent(name)}`,
				{ method: 'DELETE' },
			)
		} catch {
			// 后台回收失败无碍：文件只是残留磁盘，下次删图会跟着清
		}
	}
}

/** 编辑节点业务状态：响应式字段直接更新界面，持久化仍由编辑入口安排。 */
export function editNode<T>(nodeId: string, fn: (node: T) => void): void {
	const node = rt.editor?.getNode(nodeId) as T | undefined
	if (!node) return
	fn(node)
	scheduleSave()
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
