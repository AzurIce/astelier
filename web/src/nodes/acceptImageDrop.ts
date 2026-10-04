import { IMAGE_DRAG_MIME, IMAGE_ORDER_MIME, readImageDrag } from './dragPayload'
import type { DragImagePayload } from './dragPayload'

export interface AcceptImageOptions {
	onDrop: (images: DragImagePayload[]) => void
	onFiles?: (files: File[]) => void
	onActiveChange?: (active: boolean) => void
	activeClass?: string
}

/** 单一原生事件入口，整个内容区可接收；内部排序不进入导入流程。 */
export function acceptImageDrop(node: HTMLElement, initial: AcceptImageOptions) {
	let opts = initial
	let depth = 0
	let active = false
	function setActive(next: boolean) {
		node.classList.toggle(opts.activeClass ?? 'accepting', next)
		if (active !== next) opts.onActiveChange?.(next)
		active = next
	}
	function reset() {
		depth = 0
		setActive(false)
	}
	function accepts(e: DragEvent) {
		const types = Array.from(e.dataTransfer?.types ?? [])
		if (types.includes(IMAGE_ORDER_MIME)) return false
		return (types.includes('Files') && !!opts.onFiles) ||
			types.some((type) => [IMAGE_DRAG_MIME, 'text/uri-list', 'text/plain'].includes(type))
	}
	function enter(e: DragEvent) {
		if (!accepts(e)) return
		e.preventDefault()
		e.stopPropagation()
		depth++
		setActive(true)
	}
	function leave(e: DragEvent) {
		e.stopPropagation()
		depth = Math.max(0, depth - 1)
		if (!depth) setActive(false)
	}
	function over(e: DragEvent) {
		if (!accepts(e)) return
		e.preventDefault()
		e.stopPropagation()
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
		setActive(true)
	}
	function drop(e: DragEvent) {
		reset()
		if (!accepts(e) || !e.dataTransfer) return
		e.preventDefault()
		e.stopPropagation()
		// 文件优先，避免同一次系统拖拽的 files + URI 被重复导入。
		const files = Array.from(e.dataTransfer.files)
		if (files.length && opts.onFiles) opts.onFiles(files)
		else {
			const images = readImageDrag(e.dataTransfer)
			if (images.length) opts.onDrop(images)
		}
	}
	node.addEventListener('dragenter', enter)
	node.addEventListener('dragleave', leave)
	node.addEventListener('dragover', over)
	node.addEventListener('drop', drop)
	window.addEventListener('dragend', reset)
	window.addEventListener('drop', reset)
	return {
		update(next: AcceptImageOptions) {
			node.classList.remove(opts.activeClass ?? 'accepting')
			opts = next
			node.classList.toggle(opts.activeClass ?? 'accepting', active)
		},
		destroy() {
			reset()
			node.removeEventListener('dragenter', enter)
			node.removeEventListener('dragleave', leave)
			node.removeEventListener('dragover', over)
			node.removeEventListener('drop', drop)
			window.removeEventListener('dragend', reset)
			window.removeEventListener('drop', reset)
		},
	}
}
