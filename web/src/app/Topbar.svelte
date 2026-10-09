<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import SkinSwitcher from '../ui/theme/SkinSwitcher.svelte'
	import { design } from '../ui/theme/state.svelte'
	import type { SaveState } from '../canvas/saveQueue'
	import { graphSession } from '../canvas/session.svelte'
	import { backendRegistry } from '../backends/registry.svelte'

	// 顶栏：侧栏开关 / 来源与图名 / 保存状态 / 图包 / 主题 / 明暗 / Run
	let {
		title,
		saveState = 'saved',
		running = false,
		ready = false,
		onRename,
		onRun,
		onManage,
		sidebarOpen,
		onToggleSidebar,
		onExport,
		onImport,
		transferring = false,
	}: {
		title: string
		saveState?: SaveState
		running?: boolean
		ready?: boolean
		onRename: (title: string) => void | Promise<void>
		onRun: () => void
		onManage: () => void
		sidebarOpen: boolean
		onToggleSidebar: () => void
		onExport: () => void
		onImport: () => void
		transferring?: boolean
	} = $props()

	let editing = $state(false)
	let draft = $state('')

	function startEdit() {
		draft = title
		editing = true
	}
	async function commit() {
		editing = false
		const next = draft.trim()
		if (!next || next === title) return
		await onRename(next)
	}

	const saveText: Record<SaveState, string> = {
		saved: '已保存',
		dirty: '未保存',
		saving: '保存中',
		error: '保存失败',
	}

	let sourceName = $derived(backendRegistry.entries.find((entry) => entry.id === graphSession.backendId)?.name ?? '来源不可用')
</script>

<div class="topbar">
	<button type="button" class="ui-btn icon ghost sidebar-toggle" aria-label={sidebarOpen ? '收起侧栏' : '展开侧栏'} aria-expanded={sidebarOpen} aria-controls="workspace-sidebar" title={sidebarOpen ? '收起侧栏' : '展开侧栏'} onclick={onToggleSidebar}><Icon name={sidebarOpen ? 'sidebarCollapse' : 'sidebarExpand'} size={17} /></button>
	<span class="source-entry" title={sourceName}><Icon name="layers" size={14} /><span>{sourceName}</span></span>
	<Icon name="chevronRight" size={12} />

	{#if editing}
		<input
			class="ui-input rename"
			bind:value={draft}
			onkeydown={(e) => {
				if (e.key === 'Enter') void commit()
				else if (e.key === 'Escape') editing = false
			}}
			onblur={() => void commit()}
		/>
	{:else}
		<button type="button" class="graph-title" onclick={startEdit} disabled={!ready} title="点击重命名">
			<Icon name="graph" size={14} />
			<span class="name">{title}</span>
			<Icon name="pencil" size={12} />
		</button>
	{/if}

	<div class="save-dot {saveState}" title="画布同步状态：{saveText[saveState]}">
		<span class="dot"></span>
		<span>{saveText[saveState]}</span>
	</div>

	<div class="spacer"></div>

	<button type="button" class="ui-btn ghost sm" disabled={!ready || transferring} onclick={onImport} title="导入 .astelier 图包"><Icon name="download" size={13} />导入图</button>
	<button type="button" class="ui-btn ghost sm" disabled={!ready || transferring} onclick={onExport} title="导出当前图为 .astelier"><Icon name="upload" size={13} />导出图</button>

	<SkinSwitcher />
	{#if !sidebarOpen}
		<button type="button" class="ui-btn ghost sm" aria-label="管理后端" title="后端与 Provider 设置" onclick={onManage}><Icon name="settings" size={13} />管理</button>
	{/if}
	<IconButton
		icon={design.mode === 'dark' ? 'sun' : 'moon'}
		label={design.mode === 'dark' ? '切换到浅色' : '切换到深色'}
		size={15}
		variant="ghost"
		onclick={() => design.toggleMode()}
	/>
	<div class="divider"></div>
	<button
		type="button"
		class="ui-btn primary run-btn"
		class:running
		disabled={!ready || running}
		onclick={onRun}
		title="执行整张图（Ctrl/⌘ + Enter）"
	>
		{#if running}
			<Icon name="spinner" size={14} class="spin" />
			<span>Running…</span>
		{:else}
			<Icon name="play" size={13} />
			<span>Run</span>
		{/if}
	</button>
</div>

<style>
	.save-dot.error .dot {
		background: var(--ui-danger);
	}
	.rename {
		height: 26px;
		padding: 0 8px;
		border-color: var(--ui-accent);
		font-weight: 600;
	}
	.source-entry { display: inline-flex; align-items: center; gap: 6px; min-width: 0; max-width: 150px; color: var(--ui-dim); font-size: 12px; }
	.source-entry > :global(.ui-icon) { color: var(--ui-accent); }
	.source-entry span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
