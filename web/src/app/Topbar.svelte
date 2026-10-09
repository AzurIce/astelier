<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import SkinSwitcher from '../ui/theme/SkinSwitcher.svelte'
		import { design } from '../ui/theme/state.svelte'
	import type { SaveState } from '../canvas/saveQueue'
	import { graphSession } from '../canvas/session.svelte'
	import { backendRegistry } from '../backends/registry.svelte'

	// 顶栏：品牌 / 图名（内联改名）/ 保存状态 / 工作区 / 皮肤切换 / 明暗 / Run
	let {
		title,
		saveState = 'saved',
		running = false,
		ready = false,
		onRename,
		onRun,
		onSettings,
	}: {
		title: string
		saveState?: SaveState
		running?: boolean
		ready?: boolean
		onRename: (title: string) => void | Promise<void>
		onRun: () => void
		onSettings?: () => void
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
	<div class="brand">
		<Icon name="logo" size={17} />
		<span>Astelier</span>
	</div>
	<div class="divider"></div>

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

	<button type="button" class="ui-btn ghost sm ws-entry" onclick={onSettings} title="管理后端与 Provider"><Icon name="layers" size={13} /><span>{sourceName}</span></button>

	<SkinSwitcher />
	{#if onSettings}
		<IconButton icon="settings" label="设置" size={15} variant="ghost" onclick={onSettings} />
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
	/* 顶栏来源入口：共享 .ui-btn.ghost.sm 的底，只加固定宽度与省略 */
	.ws-entry {
		max-width: 180px;
	}
	.ws-entry :global(.ui-icon:first-child) {
		color: var(--ui-accent);
	}
	.ws-entry :global(span) {
		overflow: hidden;
		text-overflow: ellipsis;
	}
</style>
