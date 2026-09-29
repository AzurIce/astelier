// Svelte action：阻止控件上的原生 pointerdown 冒泡到 rete 的 NodeView 拖拽层。
// 必须用 action 直接 addEventListener——模板里的 onpointerdown={fn} 会被编译成
// $.delegated（委托事件），委托模式下 e.stopPropagation() 只拦 Svelte 合成事件、
// 不拦原生冒泡，节点会被滑块/下拉/按钮的按下操作拖着跑。
export function noNodeDrag(node: HTMLElement) {
	const handler = (e: PointerEvent) => e.stopPropagation()
	node.addEventListener('pointerdown', handler)
	return {
		destroy() {
			node.removeEventListener('pointerdown', handler)
		},
	}
}
