<script lang="ts">
	import { backendRegistry, connectBackend } from '../backends/registry.svelte'
	import BackendGraphs from './BackendGraphs.svelte'
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import { graphSession } from '../canvas/session.svelte'
	import { SIDEBAR_WIDTH_KEY as WIDTH_KEY, readSetting } from '../settings'
	import type { GraphDropPreview } from './graphDrop'
	let width = $state(Number(readSetting(WIDTH_KEY)) || 244)
	let resizing = $state(false)
	const children = new Map<string, BackendGraphs>()
	let { onManage, collapsed = false, dropPreview = null }: { onManage: () => void; collapsed?: boolean; dropPreview?: GraphDropPreview | null } = $props()
	$effect(() => { if (collapsed) for (const child of children.values()) child.closeMenu() })
	export async function refreshAndKeepActive() { await Promise.all([...children.values()].map((child) => child.refreshAndKeepActive())) }
	function create(e: MouseEvent, backendId: string, kind: 'graph' | 'dir') {
		e.preventDefault()
		e.stopPropagation()
		const root = (e.currentTarget as HTMLElement).closest('details')
		if (root) root.open = true
		children.get(backendId)?.startCreate(kind)
	}
	function openRootMenu(e: MouseEvent) {
		const root = (e.target as HTMLElement).closest<HTMLDetailsElement>('[data-backend-id]')
		const id = root?.dataset.backendId ?? graphSession.backendId
		const target = root ?? (e.currentTarget as HTMLElement).querySelector<HTMLDetailsElement>(`[data-backend-id="${CSS.escape(id)}"]`)
		if (target) target.open = true
		children.get(id)?.openRootMenu(e)
	}
	function resize(e: PointerEvent) {
		if (e.button !== 0) return
		e.preventDefault()
		const start = e.clientX, initial = width
		resizing = true
		const move = (ev: PointerEvent) => { width = Math.max(180, Math.min(480, initial + ev.clientX - start)) }
		const up = () => { resizing = false; localStorage.setItem(WIDTH_KEY, String(width)); window.removeEventListener('pointermove', move); window.removeEventListener('pointerup', up) }
		window.addEventListener('pointermove', move); window.addEventListener('pointerup', up)
	}
</script>
<aside id="workspace-sidebar" class="sidebar" class:resizing class:collapsed style:width="{collapsed ? 0 : width}px" style:--sidebar-width="{width}px" inert={collapsed} aria-hidden={collapsed} aria-label="后端图资源">
	<div class="sidebar-head"><div class="brand"><Icon name="logo" size={17} /><span>Astelier</span></div><button type="button" class="ui-btn ghost sm" aria-label="管理后端" title="添加与管理后端" onclick={onManage}><Icon name="settings" size={13} />管理</button></div>
	<div class="roots" oncontextmenu={openRootMenu} role="presentation">
		{#each backendRegistry.entries as entry (entry.id)}
			<details class="backend-root" class:archive-target={dropPreview?.backendId === entry.id} open data-backend-id={entry.id}>
				<summary>
					<span class="root-arrow"><Icon name="chevronRight" size={11} /></span>
					<Icon name="layers" size={14} />
					<span class="root-name" title={entry.name}>{entry.name}</span>
					{#if entry.status !== 'online'}<small>{entry.status === 'connecting' ? '连接中' : '离线'}</small>{/if}
					<span class="root-actions">
						<IconButton icon="plus" label="新建图（根目录）" sm size={14} onclick={(e) => create(e, entry.id, 'graph')} />
						<IconButton icon="folder" label="新建文件夹（根目录）" sm size={14} onclick={(e) => create(e, entry.id, 'dir')} />
					</span>
				</summary>
				{#if entry.error}<div class="error">{entry.error}</div>{/if}
				{#if entry.status !== 'online'}<button type="button" class="ui-btn ghost sm" onclick={() => void connectBackend(entry.id)}>重连</button>{/if}
				<BackendGraphs backendId={entry.id} dropPreview={dropPreview?.backendId === entry.id ? dropPreview : null} bind:this={() => children.get(entry.id), (child) => { if (child) children.set(entry.id, child); else children.delete(entry.id) }} />
			</details>
		{/each}
	</div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="resizer" onpointerdown={resize} ondblclick={() => { width = 244; localStorage.setItem(WIDTH_KEY, '244') }}></div>
</aside>
<style>
	.sidebar { position: relative; flex: none; display: flex; flex-direction: column; min-height: 0; background: var(--ui-panel); border-right: 1px solid var(--ui-border-fade); overflow: clip; transition: width 180ms var(--ui-ease); }
	.sidebar.collapsed { border-right: none; }
	.sidebar-head, .roots { width: var(--sidebar-width); }
	@media (prefers-reduced-motion: reduce) { .sidebar { transition: none; } }
	.sidebar-head { display: flex; align-items: center; justify-content: space-between; gap: 6px; height: var(--ui-app-header-h); padding: 0 12px; flex: none; border-bottom: 1px solid var(--ui-border-fade); }
	.brand > :global(.ui-icon) { color: var(--ui-accent); }
	/* 后端按内容紧凑排列，共用一个滚动区域。 */
	.roots { overflow: auto; flex: 1; min-height: 0; display: flex; flex-direction: column; gap: 12px; padding: 10px 8px; background: var(--ui-bg); }
	.backend-root { flex: none; border: 1px solid var(--ui-border-fade); border-radius: var(--ui-r-menu); background: var(--ui-panel); }
	.backend-root.archive-target { border-color: var(--ui-accent); box-shadow: 0 0 0 1px var(--ui-accent); }
	.backend-root > summary { display: flex; align-items: center; gap: 6px; padding: 6px; cursor: pointer; font-size: 12px; font-weight: 650; border-radius: var(--ui-r-menu); background: color-mix(in srgb, var(--ui-text) 4%, var(--ui-panel)); }
	.backend-root > summary:focus-visible { outline: 1px solid var(--ui-accent); outline-offset: -1px; }
	.backend-root > summary > :global(.ui-icon) { color: var(--ui-accent); }
	.root-name { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.root-actions { display: flex; gap: 1px; flex: none; }
	.root-arrow { display: flex; flex: none; color: var(--ui-faint); transition: transform var(--ui-fast) var(--ui-ease); }
	details[open] > summary .root-arrow { transform: rotate(90deg); }
	small { color: var(--ui-faint); font-size: 10px; }
	.error { padding: 8px; overflow-wrap: anywhere; color: var(--ui-danger); font-size: 10px; }
	.resizer { position: absolute; right: 0; top: 0; bottom: 0; width: 6px; cursor: col-resize; z-index: 5; }
</style>
