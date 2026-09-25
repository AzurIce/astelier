<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte'
	import { rt } from './runtime'
	import { scheduleSave } from './graphStore'
	import { nodeFactories } from './nodes/classes'

	export let open = false
	export let x = 0
	export let y = 0

	let query = ''
	let active = 0
	let inputEl: HTMLInputElement

	const labels = Object.keys(nodeFactories)

	$: items = labels.filter((l) => l.toLowerCase().includes(query.trim().toLowerCase()))

	/** 在指针处打开：清空搜索、高亮首项、聚焦输入框 */
	export async function openAt(cx: number, cy: number) {
		x = cx
		y = cy
		query = ''
		active = 0
		open = true
		await tick()
		inputEl?.focus()
	}

	function close() {
		open = false
	}

	/** 新节点落在指针指向的画布坐标（client → flow：先减容器原点再除缩放） */
	async function pick(label: string) {
		const editor = rt.editor
		const area = rt.area
		const el = document.getElementById('rete')
		const factory = nodeFactories[label]
		if (!editor || !area || !el || !factory) return
		const rect = el.getBoundingClientRect()
		const t = area.area.transform
		const node = factory()
		await editor.addNode(node)
		await area.translate(node.id, {
			x: (x - rect.left - t.x) / t.k,
			y: (y - rect.top - t.y) / t.k,
		})
		scheduleSave()
		close()
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'ArrowDown') {
			e.preventDefault()
			active = Math.min(active + 1, items.length - 1)
		} else if (e.key === 'ArrowUp') {
			e.preventDefault()
			active = Math.max(active - 1, 0)
		} else if (e.key === 'Enter') {
			e.preventDefault()
			const label = items[active]
			if (label) pick(label)
		} else if (e.key === 'Escape') {
			close()
		}
	}

	// 必须用捕获阶段监听：rete 画布的指针处理会拦截冒泡，
	// 点画布空白处的 pointerdown 不会自然冒泡到 window
	function onWindowPointerdown(e: PointerEvent) {
		if (!open) return
		if ((e.target as HTMLElement | null)?.closest?.('.an-ctx')) return
		open = false
	}

	function onWindowContextmenu() {
		if (open) open = false
	}

	onMount(() => {
		window.addEventListener('pointerdown', onWindowPointerdown, true)
		window.addEventListener('contextmenu', onWindowContextmenu, true)
	})
	onDestroy(() => {
		window.removeEventListener('pointerdown', onWindowPointerdown, true)
		window.removeEventListener('contextmenu', onWindowContextmenu, true)
	})
</script>

{#if open}
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="an-ctx"
		style:left="{x}px"
		style:top="{y}px"
		on:pointerdown|stopPropagation
		on:contextmenu|preventDefault|stopPropagation
	>
		<input
			class="an-ctx-search"
			bind:this={inputEl}
			bind:value={query}
			placeholder="搜索节点…"
			spellcheck="false"
			on:keydown={onKeydown}
			on:input={() => (active = 0)}
		/>
		<div class="an-ctx-list">
			{#each items as label, i (label)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- 键盘交互由搜索框统一处理：↑↓ 移动高亮、回车选中 -->
				<div
					class="an-ctx-item"
					class:active={i === active}
					role="menuitem"
					tabindex="-1"
					on:mouseenter={() => (active = i)}
					on:click={() => pick(label)}
				>
					{label}
				</div>
			{:else}
				<div class="an-ctx-empty">无匹配节点</div>
			{/each}
		</div>
	</div>
{/if}
