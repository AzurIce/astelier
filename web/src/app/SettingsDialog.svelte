<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { workspaceStore } from '../workspace/store'
	import { currentWorkspaceChoice } from '../workspace/selection.svelte'
	import { loadProviderConfig } from '../generation/config.svelte'
	import type { ProviderConfig, ProviderEntry } from '../workspace/types'

	// 设置：本地模式下编辑 Provider（base_url / API key / 模型列表，明文存
	// OPFS config.json，可随时清除）；远端模式下只展示工作区信息（配置在服务端）。
	let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props()

	const choice = $derived(currentWorkspaceChoice())
	const isLocal = $derived(choice.kind === 'opfs')

	let loading = $state(false)
	let saving = $state(false)
	let draft = $state<ProviderConfig>({ providers: [], active_provider: '' })
	let usage = $state<{ used: number; quota: number; persisted: boolean } | null>(null)

	$effect(() => {
		if (open && !loading) void reload()
	})

	async function reload() {
		loading = true
		try {
			if (isLocal) {
				const config = await workspaceStore().loadConfig()
				draft = { providers: config.providers.map((p) => ({ ...p, models: [...p.models] })), active_provider: config.active_provider }
				if (navigator.storage?.estimate) {
					const estimate = await navigator.storage.estimate()
					usage = {
						used: estimate.usage ?? 0,
						quota: estimate.quota ?? 0,
						persisted: navigator.storage.persisted ? await navigator.storage.persisted() : false,
					}
				}
			}
		} catch (e) {
			toast({ kind: 'err', title: '读取设置失败', msg: e instanceof Error ? e.message : String(e) })
		} finally {
			loading = false
		}
	}

	function fmtSize(n: number): string {
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`
		if (n < 1024 * 1024 * 1024) return `${(n / 1048576).toFixed(1)} MB`
		return `${(n / 1073741824).toFixed(2)} GB`
	}

	function addProvider() {
		const id = `provider-${draft.providers.length + 1}`
		draft.providers.push({ id, name: '新 Provider', base_url: '', api_key: '', models: [], overrides: {} })
		draft.providers = [...draft.providers]
		if (!draft.active_provider) draft.active_provider = id
	}

	function removeProvider(id: string) {
		draft.providers = draft.providers.filter((p) => p.id !== id)
		if (draft.active_provider === id) draft.active_provider = draft.providers[0]?.id ?? ''
	}

	function setActive(id: string) {
		draft.active_provider = id
	}

	function providerValid(p: ProviderEntry): boolean {
		return Boolean(p.id.trim() && p.name.trim() && p.base_url.trim())
	}

	async function save() {
		if (saving) return
		for (const p of draft.providers) {
			if (!providerValid(p)) {
				toast({ kind: 'err', title: 'Provider 信息不完整', msg: `${p.name || p.id || '未命名'}：id、名称与 Base URL 均必填` })
				return
			}
		}
		if (draft.providers.length && !draft.providers.some((p) => p.id === draft.active_provider)) {
			draft.active_provider = draft.providers[0].id
		}
		saving = true
		try {
			await workspaceStore().saveConfig(draft)
			await loadProviderConfig()
			toast({ kind: 'ok', title: '设置已保存', msg: 'Provider 配置已写入本地工作区' })
			onclose()
		} catch (e) {
			toast({ kind: 'err', title: '保存设置失败', msg: e instanceof Error ? e.message : String(e) })
		} finally {
			saving = false
		}
	}

	async function requestPersist() {
		try {
			const granted = await navigator.storage.persist()
			usage = usage ? { ...usage, persisted: granted } : usage
			toast({
				kind: granted ? 'ok' : 'info',
				title: granted ? '已开启持久存储' : '浏览器未授予持久存储',
				msg: granted ? '站点数据被自动清理的风险降低' : '存储仍可能被浏览器在空间不足时清理',
			})
		} catch {
			toast({ kind: 'err', title: '申请持久存储失败' })
		}
	}
</script>

{#if open}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div
		class="ui-overlay"
		role="presentation"
		onclick={(e) => {
			if (e.target === e.currentTarget) onclose()
		}}
		onkeydown={(e) => {
			if (e.key === 'Escape') onclose()
		}}
	>
		<div class="ui-dialog settings-dialog" role="dialog" aria-modal="true" aria-label="设置">
			<header>
				<Icon name="settings" size={16} />
				设置
				<button type="button" class="close" onclick={onclose} aria-label="关闭"><Icon name="x" size={14} /></button>
			</header>

			<div class="body">
				{#if loading}
					<div class="loading"><Icon name="spinner" size={16} class="spin" />读取中…</div>
				{:else if !isLocal}
					<section class="card">
						<h3>工作区</h3>
						<p>当前使用<b>远端工作区</b>：<code>{choice.kind === 'http' ? choice.baseUrl : ''}</code></p>
						<p class="hint">Provider 与密钥配置在服务端（data/config.json）；生成请求由服务端代发。本地 Provider 设置请切回本地工作区。</p>
					</section>
				{:else}
					<section class="card">
						<h3>工作区存储</h3>
						{#if usage}
							<div class="storage-row">
								<span class="mono">{fmtSize(usage.used)}{usage.quota ? ` / ${fmtSize(usage.quota)}` : ''}</span>
								<span class="badge" class:on={usage.persisted}>{usage.persisted ? '持久存储' : '可被清理'}</span>
								{#if !usage.persisted}
									<button type="button" class="ui-btn ghost sm" onclick={() => void requestPersist()}>申请持久存储</button>
								{/if}
							</div>
							{#if usage.quota}
								<div class="quota-bar"><div class="quota-fill" style="width:{Math.min(100, (usage.used / usage.quota) * 100)}%"></div></div>
							{/if}
							<p class="hint">数据保存在本浏览器的 OPFS；清除站点数据会一并删除。导出备份见「工作区导入/导出」。</p>
						{:else}
							<p class="hint">当前浏览器不支持存储配额查询。</p>
						{/if}
					</section>

					<section class="card">
						<div class="card-head">
							<h3>Provider</h3>
							<button type="button" class="ui-btn ghost sm" onclick={addProvider}><Icon name="plus" size={12} />新增</button>
						</div>
						<p class="hint">API Key 明文保存在本机 OPFS（可在下方随时清除），仅本地工作区的直连生图使用。</p>
						{#each draft.providers as provider, index (provider.id + index)}
							<div class="provider" class:active={provider.id === draft.active_provider}>
								<div class="provider-head">
									<label class="radio" title="设为当前激活的 Provider">
										<input type="radio" name="active-provider" checked={provider.id === draft.active_provider} onchange={() => setActive(provider.id)} />
										<input class="name" bind:value={provider.name} placeholder="名称" aria-label="Provider 名称" />
									</label>
									<code class="pid" title="Provider id（建图与执行时引用，创建后不可改）">{provider.id}</code>
									<button type="button" class="ui-btn ghost sm danger" onclick={() => removeProvider(provider.id)} title="删除该 Provider"><Icon name="trash" size={12} /></button>
								</div>
								<div class="provider-fields">
									<label>
										<span>Base URL</span>
										<input bind:value={provider.base_url} placeholder="https://api.openai.com/v1" spellcheck="false" />
									</label>
									<label>
										<span>API Key</span>
										<div class="key-row">
											<input type="password" bind:value={provider.api_key} placeholder="sk-…" spellcheck="false" autocomplete="off" />
											{#if provider.api_key}
												<button type="button" class="ui-btn ghost sm" onclick={() => (provider.api_key = '')} title="清除密钥">清除</button>
											{/if}
										</div>
									</label>
									<label>
										<span>模型列表（每行一个）</span>
										<textarea
											rows="3"
											spellcheck="false"
											value={provider.models.join('\n')}
											oninput={(e) => {
												provider.models = e.currentTarget.value.split('\n').map((s) => s.trim()).filter(Boolean)
											}}
										></textarea>
									</label>
								</div>
							</div>
						{/each}
						{#if !draft.providers.length}
							<p class="hint">还没有 Provider，点「新增」添加一个。</p>
						{/if}
					</section>
				{/if}
			</div>

			{#if isLocal && !loading}
				<footer>
					<button type="button" class="ui-btn ghost" onclick={onclose}>取消</button>
					<button type="button" class="ui-btn primary" disabled={saving} onclick={() => void save()}>
						{#if saving}<Icon name="spinner" size={13} class="spin" />{/if}保存
					</button>
				</footer>
			{/if}
		</div>
	</div>
{/if}

<style>
	.settings-dialog {
		width: min(560px, calc(100vw - 48px));
		max-height: min(720px, calc(100vh - 64px));
		display: flex;
		flex-direction: column;
	}
	header {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	header .close {
		margin-left: auto;
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border: none;
		border-radius: var(--ui-r-control);
		background: none;
		color: var(--ui-dim);
		cursor: pointer;
	}
	header .close:hover {
		background: var(--ui-input);
		color: var(--ui-text);
	}
	.settings-dialog :global(.body) {
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		gap: 12px;
	}
	.loading {
		display: flex;
		align-items: center;
		gap: 8px;
		color: var(--ui-dim);
		font-size: 12px;
		padding: 12px 0;
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 12px;
		border: 1px solid var(--ui-border-fade);
		border-radius: 10px;
		background: var(--ui-card);
	}
	.card h3 {
		margin: 0;
		font-size: 12px;
		font-weight: 650;
		color: var(--ui-text);
	}
	.card-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
	}
	.card p {
		margin: 0;
		font-size: 11.5px;
		color: var(--ui-dim);
		overflow-wrap: anywhere;
	}
	.card p code {
		font-family: var(--ui-mono, monospace);
		color: var(--ui-accent);
	}
	.hint {
		font-size: 10.5px;
		color: var(--ui-faint);
	}
	.storage-row {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.storage-row .mono {
		font-family: var(--ui-mono, monospace);
		font-size: 11px;
		color: var(--ui-text);
	}
	.badge {
		padding: 1px 7px;
		border-radius: 999px;
		border: 1px solid var(--ui-border-fade);
		font-size: 9.5px;
		color: var(--ui-faint);
	}
	.badge.on {
		color: var(--ui-accent);
		border-color: var(--ui-accent-fade);
	}
	.quota-bar {
		height: 4px;
		border-radius: 2px;
		background: var(--ui-track);
		overflow: hidden;
	}
	.quota-fill {
		height: 100%;
		background: var(--ui-accent);
	}
	.provider {
		display: flex;
		flex-direction: column;
		gap: 8px;
		padding: 10px;
		border: 1px solid var(--ui-border-fade);
		border-radius: 9px;
		background: var(--ui-input);
	}
	.provider.active {
		border-color: var(--ui-accent-fade);
	}
	.provider-head {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.provider-head .radio {
		display: flex;
		align-items: center;
		gap: 7px;
		flex: 1;
		min-width: 0;
	}
	.provider-head .radio input[type='radio'] {
		accent-color: var(--ui-accent);
	}
	.provider-head .name {
		flex: 1;
		min-width: 0;
		height: 26px;
		padding: 0 8px;
		border: 1px solid var(--ui-border-fade);
		border-radius: var(--ui-r-control);
		background: var(--ui-card);
		color: var(--ui-text);
		font: inherit;
		font-size: 11.5px;
		outline: none;
	}
	.provider-head .name:focus {
		border-color: var(--ui-accent);
	}
	.pid {
		font-family: var(--ui-mono, monospace);
		font-size: 10px;
		color: var(--ui-faint);
	}
	.provider-fields {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}
	.provider-fields label {
		display: flex;
		flex-direction: column;
		gap: 4px;
		font-size: 10.5px;
		color: var(--ui-dim);
	}
	.provider-fields input,
	.provider-fields textarea {
		height: 26px;
		padding: 0 8px;
		border: 1px solid var(--ui-border-fade);
		border-radius: var(--ui-r-control);
		background: var(--ui-card);
		color: var(--ui-text);
		font: inherit;
		font-size: 11.5px;
		outline: none;
	}
	.provider-fields textarea {
		height: auto;
		padding: 6px 8px;
		resize: vertical;
		font-family: var(--ui-mono, monospace);
		font-size: 11px;
		line-height: 1.5;
	}
	.provider-fields input:focus,
	.provider-fields textarea:focus {
		border-color: var(--ui-accent);
	}
	.key-row {
		display: flex;
		gap: 6px;
	}
	.key-row input {
		flex: 1;
		min-width: 0;
		font-family: var(--ui-mono, monospace);
	}
	.danger:hover {
		color: var(--ui-danger);
	}
</style>
