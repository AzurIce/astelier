<script lang="ts">
	import { tick } from 'svelte'
	import { rt } from './runtime'
	import { scheduleSave } from './persist'
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

	function onWindowPointerdown(e: PointerEvent) {
		if (!open) return
		const target = e.target as HTMLElement | null
		if (!target?.closest('.an-ctx')) close()
	}
</script>

<svelte:window on:pointerdown={onWindowPointerdown} />

{#if open}
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
				<div
					class="an-ctx-item"
					class:active={i === active}
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
