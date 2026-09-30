// Svelte action：隔离控件内滚轮与画布缩放。
// rete 的 Zoom 监听在画布容器上（wheel 冒泡即缩放），导致节点内可滚动
// 区域（textarea 等）的滚轮被画布吞掉。这里：内容还可沿该方向滚时
// stopPropagation（只滚内容）；已到顶/底则放行，让画布正常缩放。
// 必须用 action 直接 addEventListener——Svelte 模板里的 onwheel 会被
// 编译成委托事件，stopPropagation 拦不住原生冒泡。
export function noCanvasWheel(node: HTMLElement) {
	const onWheel = (e: WheelEvent) => {
		const canScroll = node.scrollHeight > node.clientHeight + 1
		if (!canScroll) return
		const atTop = node.scrollTop <= 0
		const atBottom = node.scrollTop + node.clientHeight >= node.scrollHeight - 1
		const goingUp = e.deltaY < 0
		if (!((goingUp && atTop) || (!goingUp && atBottom))) {
			e.stopPropagation()
		}
	}
	node.addEventListener('wheel', onWheel, { passive: true })
	return {
		destroy() {
			node.removeEventListener('wheel', onWheel)
		},
	}
}
