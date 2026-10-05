import { rt } from './runtime'

/* ---------------- 指针广播（伪连线实时跟随） ---------------- */

const pointerSubs = new Set<(p: { x: number; y: number }) => void>()

/** 订阅画布指针（内容坐标）变化；伪连线拖拽时每帧回调 */
export function subscribePointer(cb: (p: { x: number; y: number }) => void): () => void {
	pointerSubs.add(cb)
	return () => pointerSubs.delete(cb)
}

export function notifyPointer(p: { x: number; y: number }) {
	for (const cb of pointerSubs) cb(p)
}

/* ---------------- 连线选中（点击高亮 + Delete 删除） ---------------- */
const connSubs = new Set<(id: string | null) => void>()
let selectedConnection: string | null = null

export function subscribeConnection(cb: (id: string | null) => void): () => void {
	connSubs.add(cb)
	cb(selectedConnection)
	return () => connSubs.delete(cb)
}

export function selectConnection(id: string | null): void {
	if (id === selectedConnection) return
	selectedConnection = id
	for (const cb of connSubs) cb(id)
}

/** 删除当前选中的连线；无选中返回 false */
export async function removeSelectedConnection(): Promise<boolean> {
	if (!selectedConnection || !rt.editor) return false
	const id = selectedConnection
	selectConnection(null)
	await rt.editor.removeConnection(id)
	return true
}

