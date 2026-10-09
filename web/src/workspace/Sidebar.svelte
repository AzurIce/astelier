<script lang="ts">
	import { backendRegistry, connectBackend } from '../backends/registry.svelte'
	import BackendGraphs from './BackendGraphs.svelte'
	import Icon from '../ui/Icon.svelte'
	import { SIDEBAR_WIDTH_KEY as WIDTH_KEY, readSetting } from '../settings'
	let width = $state(Number(readSetting(WIDTH_KEY)) || 244)
	let resizing = $state(false)
	const children = new Map<string, BackendGraphs>()
	let { onManage }: { onManage: () => void } = $props()
	export async function refreshAndKeepActive() { await Promise.all([...children.values()].map((child) => child.refreshAndKeepActive())) }
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
<aside class="sidebar" class:resizing style:width="{width}px" aria-label="后端图资源">
	<div class="side-head"><strong>后端</strong><button type="button" class="ui-btn ghost sm" onclick={onManage}><Icon name="plus" size={13} />添加 / 管理</button></div>
	<div class="roots">
		{#each backendRegistry.entries as entry (entry.id)}
			<details class="backend-root" open data-backend-id={entry.id}>
				<summary><span class="root-arrow"><Icon name="chevronRight" size={11} /></span><Icon name="layers" size={14} /><span>{entry.name}</span><small>{entry.status === 'online' ? '' : entry.status === 'connecting' ? '连接中' : '离线'}</small></summary>
				{#if entry.error}<div class="error">{entry.error}</div>{/if}
				{#if entry.status !== 'online'}<button type="button" class="ui-btn ghost sm" onclick={() => void connectBackend(entry.id)}>重连</button>{/if}
				<BackendGraphs backendId={entry.id} bind:this={() => children.get(entry.id), (child) => { if (child) children.set(entry.id, child); else children.delete(entry.id) }} />
			</details>
		{/each}
	</div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="resizer" onpointerdown={resize} ondblclick={() => { width = 244; localStorage.setItem(WIDTH_KEY, '244') }}></div>
</aside>
<style>
	.sidebar { position: relative; flex: none; display: flex; flex-direction: column; min-height: 0; background: var(--ui-panel); border-right: 1px solid var(--ui-border-fade); }
	.side-head { display: flex; align-items: center; justify-content: space-between; gap: 6px; padding: 8px 10px; flex: none; font-size: 12px; }
	/* 折叠外层只负责滚动；每棵图树（.backend-graphs）自己撑满 remaining 高度 */
	.roots { overflow: auto; flex: 1; min-height: 0; display: flex; flex-direction: column; }
	/* 收起的后端只占标题一行；展开的后端各自分高度，多后端时内部再滚动 */
	.backend-root { flex: none; }
	.backend-root[open] { display: flex; flex-direction: column; flex: 1 1 0; min-height: 0; overflow: hidden; }
	.backend-root > summary { display: flex; align-items: center; gap: 7px; padding: 9px; cursor: pointer; font-size: 12px; }
	.backend-root > summary > span:not(.root-arrow) { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.root-arrow { display: flex; flex: none; color: var(--ui-faint); transition: transform var(--ui-fast) var(--ui-ease); }
	details[open] > summary .root-arrow { transform: rotate(90deg); }
	small { color: var(--ui-faint); font-size: 10px; }
	.error { padding: 8px; overflow-wrap: anywhere; color: var(--ui-danger); font-size: 10px; }
	.resizer { position: absolute; right: -3px; top: 0; bottom: 0; width: 6px; cursor: col-resize; z-index: 5; }
</style>
