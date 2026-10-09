// Svelte action：隔离控件内滚轮与画布缩放。
// rete 的 Zoom 监听在画布容器上（wheel 冒泡即缩放），导致节点内可滚动
// 区域（textarea 等）的滚轮被画布吞掉。无论是否到达边界都只滚内容。
// 必须用 action 直接 addEventListener——Svelte 模板里的 onwheel 会被
// 编译成委托事件，stopPropagation 拦不住原生冒泡。
export function noCanvasWheel(node: HTMLElement) {
	const onWheel = (e: WheelEvent) => e.stopPropagation()
	node.addEventListener('wheel', onWheel, { passive: true })
	return {
		destroy() {
			node.removeEventListener('wheel', onWheel)
		},
	}
}
