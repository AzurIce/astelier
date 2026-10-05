<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import SkinSwitcher from '../ui/theme/SkinSwitcher.svelte'
	import Popover from '../ui/Popover.svelte'
	import { design } from '../ui/theme/state.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import type { SaveState } from '../canvas/saveQueue'
	import { flushNow } from '../canvas/session.svelte'
	import { saveWorkspaceChoice, validateRemoteWorkspace, workspaceChoice, type WorkspaceChoice } from '../workspace/selection.svelte'

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

	// ---------- 工作区切换 ----------
	let wsOpen = $state(false)
	let wsAnchor = $state({ x: 0, y: 0 })
	let remoteUrl = $state('')
	let connecting = $state(false)
	const choice = $derived(workspaceChoice.current)
	const workspaceText = $derived(choice.kind === 'opfs' ? '本地' : '远端')

	function openWorkspace(e: MouseEvent) {
		const rect = (e.currentTarget as HTMLElement).getBoundingClientRect()
		wsAnchor = { x: rect.right, y: rect.bottom + 6 }
		remoteUrl = choice.kind === 'http' ? choice.baseUrl : ''
		wsOpen = true
	}

	/** 切换前必须把当前图的待保存修改写完；失败则留在原工作区 */
	async function switchTo(next: WorkspaceChoice) {
		if (next.kind === choice.kind && (next.kind === 'opfs' || next.baseUrl === (choice as { baseUrl: string }).baseUrl)) {
			wsOpen = false
			return
		}
		try {
			await flushNow()
		} catch {
			toast({ kind: 'err', title: '有修改尚未保存', msg: '请先重试保存（保存失败时不能切换工作区）' })
			return
		}
		saveWorkspaceChoice(next)
		location.reload()
	}

	async function connectRemote() {
		const url = remoteUrl.trim()
		if (!url || connecting) return
		connecting = true
		try {
			await validateRemoteWorkspace(url)
			await switchTo({ kind: 'http', baseUrl: url })
		} catch (e) {
			toast({ kind: 'err', title: '连接远端失败', msg: e instanceof Error ? e.message : String(e) })
		} finally {
			connecting = false
		}
	}
</script>

<div class="topbar">
	<div class="brand">
		<Icon name="logo" size={17} />
		<span>Atelier</span>
	</div>
	<div class="divider"></div>

	{#if editing}
		<input
			class="rename"
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

	<button
		type="button"
		class="ws-entry"
		onclick={openWorkspace}
		title={choice.kind === 'opfs' ? '本地工作区（OPFS）· 点击切换' : `${choice.baseUrl} · 点击切换`}
	>
		<Icon name="layers" size={13} />
		<span>{workspaceText}</span>
		<Icon name="chevronDown" size={11} />
	</button>
	<Popover bind:open={wsOpen} anchor={wsAnchor} width={288} onclose={() => (wsOpen = false)}>
		<div class="ws-pop">
			<button type="button" class="ws-opt" class:active={choice.kind === 'opfs'} onclick={() => void switchTo({ kind: 'opfs' })}>
				<Icon name="layers" size={15} />
				<span class="meta"><strong>本地工作区</strong><small>OPFS · 数据保存在本浏览器</small></span>
				{#if choice.kind === 'opfs'}<Icon name="check" size={14} />{/if}
			</button>
			<div class="ws-sep"></div>
			<div class="ws-remote">
				<span class="ws-remote-title"><Icon name="graph" size={13} />远端服务</span>
				{#if choice.kind === 'http'}<small class="ws-current">{choice.baseUrl}</small>{/if}
				<div class="ws-url">
					<input
						bind:value={remoteUrl}
						placeholder="http://127.0.0.1:8230"
						spellcheck="false"
						aria-label="远端服务地址"
						onkeydown={(e) => {
							if (e.key === 'Enter') void connectRemote()
						}}
					/>
					<button type="button" class="ui-btn ghost sm" disabled={connecting || !remoteUrl.trim()} onclick={() => void connectRemote()}>
						{#if connecting}<Icon name="spinner" size={12} class="spin" />{:else}连接{/if}
					</button>
				</div>
				<small class="ws-hint">数据与生图都在服务端；需服务端允许跨域访问</small>
			</div>
		</div>
	</Popover>

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
		border: 1px solid var(--ui-accent);
		border-radius: var(--ui-r-control);
		background: var(--ui-input);
		color: var(--ui-text);
		font: inherit;
		font-weight: 600;
		outline: none;
	}
	.ws-entry {
		display: flex;
		align-items: center;
		gap: 5px;
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--ui-border-fade);
		border-radius: var(--ui-r-control);
		background: var(--ui-input);
		color: var(--ui-dim);
		font: inherit;
		font-size: 11px;
		cursor: pointer;
	}
	.ws-entry:hover {
		color: var(--ui-text);
		border-color: var(--ui-accent-fade);
	}
	.ws-entry :global(.ui-icon:first-child) {
		color: var(--ui-accent);
	}
	.ws-pop {
		display: flex;
		flex-direction: column;
		gap: 4px;
		padding: 6px;
	}
	.ws-opt {
		display: flex;
		align-items: center;
		gap: 9px;
		width: 100%;
		padding: 7px 8px;
		border: none;
		border-radius: var(--ui-r-control);
		background: none;
		color: var(--ui-text);
		font: inherit;
		font-size: 12px;
		text-align: left;
		cursor: pointer;
	}
	.ws-opt:hover {
		background: var(--ui-input);
	}
	.ws-opt.active {
		background: var(--ui-accent-weak);
	}
	.ws-opt.active :global(.ui-icon:last-child) {
		margin-left: auto;
		color: var(--ui-accent);
	}
	.ws-opt .meta {
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.ws-opt small {
		font-size: 10px;
		color: var(--ui-faint);
	}
	.ws-sep {
		height: 1px;
		margin: 2px 4px;
		background: var(--ui-border-fade);
	}
	.ws-remote {
		display: flex;
		flex-direction: column;
		gap: 6px;
		padding: 4px 8px 6px;
	}
	.ws-remote-title {
		display: flex;
		align-items: center;
		gap: 6px;
		font-size: 12px;
		font-weight: 600;
		color: var(--ui-text);
	}
	.ws-current {
		font-family: var(--ui-mono, monospace);
		font-size: 10px;
		color: var(--ui-accent);
		overflow-wrap: anywhere;
	}
	.ws-url {
		display: flex;
		gap: 6px;
	}
	.ws-url input {
		flex: 1;
		min-width: 0;
		height: 26px;
		padding: 0 8px;
		border: 1px solid var(--ui-border-fade);
		border-radius: var(--ui-r-control);
		background: var(--ui-input);
		color: var(--ui-text);
		font-family: var(--ui-mono, monospace);
		font-size: 11px;
		outline: none;
	}
	.ws-url input:focus {
		border-color: var(--ui-accent);
	}
	.ws-hint {
		font-size: 10px;
		color: var(--ui-faint);
	}
</style>
