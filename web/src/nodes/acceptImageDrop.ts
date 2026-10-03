// 画布上「接收 store 图片」的共用 Svelte action。
// 挂在节点的图片输入区域（LoadImage body / Generate / Preview）。
// - H5 DnD（dragover/drop）：放行 preventDefault，从 dataTransfer 读图片 URL
// - 视觉：dragover 期间给宿主加 class
import type { DragImagePayload } from './dragPayload'

/** Svelte action 的最小类型（避免依赖 legacy 导出） */
type ActionReturn = { destroy?: () => void }
type Action<Node extends Element, Param> = (node: Node, param: Param) => ActionReturn

export interface AcceptImageOptions {
	/** 落下时调用；返回是否接受（用于 toast 反馈） */
	onDrop: (img: DragImagePayload) => void | Promise<void>
	/** dragover 期间加在宿主上的 class（默认 accepting） */
	activeClass?: string
}

export const acceptImageDrop: Action<HTMLElement, AcceptImageOptions> = (node, opts) => {
	let inside = 0

	function readPayload(e: DragEvent): DragImagePayload | null {
		const uri = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain')
		if (!uri) return null
		return { kind: 'store', url: uri, store: '', file: uri.split('/').pop() ?? '' }
	}

	function onEnter(e: DragEvent) {
		e.preventDefault()
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
		inside++
		node.classList.add(opts.activeClass ?? 'accepting')
	}
	function onLeave() {
		inside = Math.max(0, inside - 1)
		if (inside === 0) node.classList.remove(opts.activeClass ?? 'accepting')
	}
	function onOver(e: DragEvent) {
		e.preventDefault()
		if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy'
	}
	async function onDropEvt(e: DragEvent) {
		e.preventDefault()
		inside = 0
		node.classList.remove(opts.activeClass ?? 'accepting')
		const img = readPayload(e)
		if (!img || !img.url) return
		await opts.onDrop(img)
	}

	node.addEventListener('dragenter', onEnter)
	node.addEventListener('dragleave', onLeave)
	node.addEventListener('dragover', onOver)
	node.addEventListener('drop', onDropEvt)
	return {
		destroy() {
			node.removeEventListener('dragenter', onEnter)
			node.removeEventListener('dragleave', onLeave)
			node.removeEventListener('dragover', onOver)
			node.removeEventListener('drop', onDropEvt)
		},
	}
}

