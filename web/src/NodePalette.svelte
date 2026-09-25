<script lang="ts">
	import { tick } from 'svelte'
	import Popover from './components/Popover.svelte'
	import Icon from './components/Icon.svelte'
	import type { IconName } from './design/icons'
	import { rt } from './runtime'
	import { scheduleSave } from './graphStore'
	import { factoriesByType, type NodeType } from './nodes/classes'

	interface NodeDef {
		type: NodeType
		label: string
		desc: string
		icon: IconName
		group: string
	}

	const CATALOG: NodeDef[] = [
		{ type: 'model', label: 'Model', desc: '选择 Provider 与模型', icon: 'model', group: '输入' },
		{ type: 'prompt', label: 'Prompt', desc: '输入提示词', icon: 'prompt', group: '输入' },
		{ type: 'image', label: 'Image', desc: '上传参考图 / 垫图', icon: 'image', group: '输入' },
		{ type: 'generate', label: 'Generate', desc: '按协议参数调用模型', icon: 'generate', group: '生成' },
		{ type: 'preview', label: 'Preview', desc: '展示生成结果', icon: 'preview', group: '输出' },
	]

	let open = $state(false)
	let anchor = $state({ x: 0, y: 0 })
	let query = $state('')
	let active = $state(0)
	let inputEl: HTMLInputElement | undefined = $state()

	/** 在指针处打开：清空搜索、高亮首项、聚焦输入框 */
	export async function openAt(cx: number, cy: number) {
		anchor = { x: cx, y: cy }
		query = ''
		active = 0
		open = true
		await tick()
		inputEl?.focus()
		inputEl?.select()
	}

	function close() {
		open = false
	}

	let items = $derived.by(() => {
		const q = query.trim().toLowerCase()
		if (!q) return CATALOG
		return CATALOG.filter(
			(d) =>
				d.label.toLowerCase().includes(q) ||
				d.desc.toLowerCase().includes(q) ||
				d.type.includes(q),
		)
	})
	let groups = $derived.by(() => {
		const map = new Map<string, NodeDef[]>()
		for (const it of items) {
			if (!map.has(it.group)) map.set(it.group, [])
			map.get(it.group)!.push(it)
		}
		return [...map.entries()]
	})

	$effect(() => {
		// 过滤结果变化时把高亮收回首项
		void items
		active = 0
	})

	/** 新节点落在指针指向的画布坐标（client → flow：先减容器原点再除缩放） */
	async function pick(def: NodeDef) {
		const editor = rt.editor
		const area = rt.area
		const el = document.getElementById('rete')
		const factory = factoriesByType[def.type]
		if (!editor || !area || !el || !factory) return
		const rect = el.getBoundingClientRect()
		const t = area.area.transform
		const node = factory()
		await editor.addNode(node)
		await area.translate(node.id, {
			x: (anchor.x - rect.left - t.x) / t.k,
			y: (anchor.y - rect.top - t.y) / t.k,
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
			const def = items[active]
			if (def) void pick(def)
		}
	}
</script>

<Popover bind:open {anchor} align="start" width={304} panelClass="ui-palette" onclose={close}>
	<div class="search-row">
		<Icon name="search" size={15} />
		<input
			bind:this={inputEl}
			bind:value={query}
			placeholder="搜索节点…"
			spellcheck="false"
			onkeydown={onKeydown}
		/>
		<span class="ui-kbd">Esc</span>
	</div>

	<div class="list">
		{#each groups as [group, defs] (group)}
			<div class="ui-menu-label">{group}</div>
			{#each defs as def (def.type)}
				{@const idx = items.indexOf(def)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<button
					type="button"
					class="ui-palette-item"
					class:active={idx === active}
					onmouseenter={() => (active = idx)}
					onclick={() => void pick(def)}
				>
					<span class="type-icon"><Icon name={def.icon} size={14} /></span>
					<span class="meta">
						<strong>{def.label}</strong>
						<small>{def.desc}</small>
					</span>
				</button>
			{/each}
		{:else}
			<div class="ui-palette-empty">没有匹配的节点</div>
		{/each}
	</div>

	<div class="palette-foot">
		<span><span class="ui-kbd">↑</span><span class="ui-kbd">↓</span> 选择</span>
		<span><span class="ui-kbd">↵</span> 添加</span>
	</div>
</Popover>
