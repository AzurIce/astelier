<script lang="ts">
	import { tick, type Snippet } from 'svelte'

	// 浮层容器：负责定位翻转 / 点击外部关闭 / Esc 关闭。
	// 定位由调用方给锚点 client 坐标，打开后测量自身尺寸再翻转到可视区内。
	let {
		open = $bindable(false),
		anchor = { x: 0, y: 0 },
		align = 'end',
		width,
		panelClass = '',
		onclose,
		children,
	}: {
		open?: boolean
		anchor?: { x: number; y: number }
		align?: 'start' | 'end'
		width?: number
		panelClass?: string
		onclose?: () => void
		children: Snippet
	} = $props()

	let el: HTMLElement | null = $state(null)
	let pos = $state({ left: 0, top: 0 })

	function place() {
		if (!el) return
		const w = el.offsetWidth
		const h = el.offsetHeight
		const vw = window.innerWidth
		const vh = window.innerHeight
		const pad = 10
		let left = align === 'end' ? anchor.x - w : anchor.x
		left = Math.min(left, vw - w - pad)
		left = Math.max(left, pad)
		let top = anchor.y
		if (top + h > vh - pad) top = Math.max(pad, anchor.y - h)
		pos = { left, top }
	}

	$effect(() => {
		if (!open) return
		void tick().then(place)
		window.addEventListener('resize', place)
		window.addEventListener('pointerdown', onPointerdown, true)
		window.addEventListener('keydown', onKeydown, true)
		window.addEventListener('contextmenu', onContextmenu, true)
		return () => {
			window.removeEventListener('resize', place)
			window.removeEventListener('pointerdown', onPointerdown, true)
			window.removeEventListener('keydown', onKeydown, true)
			window.removeEventListener('contextmenu', onContextmenu, true)
		}
	})

	function close() {
		open = false
		onclose?.()
	}
	function onPointerdown(e: PointerEvent) {
		if (!(e.target as HTMLElement | null)?.closest?.('.ui-popover')) close()
	}
	function onContextmenu(e: MouseEvent) {
		if (!(e.target as HTMLElement | null)?.closest?.('.ui-popover')) close()
	}
	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation()
			close()
		}
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		bind:this={el}
		class="ui-popover {panelClass}"
		role="dialog"
		style:left="{pos.left}px"
		style:top="{pos.top}px"
		style:width={width ? `${width}px` : undefined}
	>
		{@render children()}
	</div>
{/if}
