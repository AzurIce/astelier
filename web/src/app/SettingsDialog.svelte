<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { untrack } from 'svelte'
	import { backendRegistry, addServer, connectBackend, refreshProviders } from '../backends/registry.svelte'
	import { detachBackend, flushNow, updateBackendConnection } from '../canvas/session.svelte'
	import type { BackendConnection } from '../backends/types'
	import { loadLocalConfig, saveLocalConfig } from '../generation/localConfig'
	import { exportWorkspaceZip, importWorkspaceZip } from '../workspace/opfs/transfer'
	import type { ProviderConfig, ProviderEntry } from '../generation/localConfig'

	// 设备级后端管理、本地 Provider 配置和 OPFS 备份。
	let { open = false, onclose }: { open?: boolean; onclose: () => void } = $props()

	let serverName = $state('')
	let serverUrl = $state('')
	let connecting = $state(false)
	let connectionDraft = $state<{ id: string; name: string; baseUrl: string } | null>(null)
	let updatingConnection = $state(false)
	function editConnection(entry: BackendConnection) { connectionDraft = { id: entry.id, name: entry.name, baseUrl: entry.baseUrl ?? '' } }
	async function updateConnection() {
		if (!connectionDraft || updatingConnection) return
		const next = { ...connectionDraft }
		updatingConnection = true
		try { await updateBackendConnection(next.id, next.name, next.baseUrl); connectionDraft = null; toast({ kind: 'ok', title: '后端连接已更新' }) }
		catch (error) { toast({ kind: 'err', title: '修改连接失败', msg: String(error) }) }
		finally { updatingConnection = false }
	}

	/** 后端状态 → 徽章文案与色调（与 .ui-badge 变体同名） */
	function statusOf(status: string): { label: string; tone: 'ok' | 'warn' | 'err' } {
		if (status === 'online') return { label: '已连接', tone: 'ok' }
		if (status === 'connecting') return { label: '连接中', tone: 'warn' }
		return { label: '离线', tone: 'err' }
	}

	async function addConnection() {
		if (connecting) return
		connecting = true
		try { await addServer(serverName, serverUrl); serverName = ''; serverUrl = '' }
		catch (error) { toast({ kind: 'err', title: '添加后端失败', msg: String(error) }) }
		finally { connecting = false }
	}
	async function detach(id: string) {
		try { await detachBackend(id) } catch (error) { toast({ kind: 'err', title: '移除连接失败', msg: String(error) }) }
	}

	let loading = $state(false)
	let saving = $state(false)
	let draft = $state<ProviderConfig>({ providers: [], active_provider: '' })
	let usage = $state<{ used: number; quota: number; persisted: boolean } | null>(null)
	let transferring = $state(false)
	let importInput = $state<HTMLInputElement>()

	$effect(() => {
		if (open) untrack(() => void reload())
		else connectionDraft = null
	})

	async function reload() {
		loading = true
		try {
			const config = await loadLocalConfig()
			draft = { providers: config.providers.map((p) => ({ ...p, models: [...p.models] })), active_provider: config.active_provider }
			if (navigator.storage?.estimate) {
				const estimate = await navigator.storage.estimate()
				usage = {
					used: estimate.usage ?? 0,
					quota: estimate.quota ?? 0,
					persisted: navigator.storage.persisted ? await navigator.storage.persisted() : false,
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
		const id = `provider-${crypto.randomUUID()}`
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
			await saveLocalConfig($state.snapshot(draft))
			await refreshProviders('local')
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

	async function exportZip() {
		if (transferring) return
		transferring = true
		try {
			await flushNow()
			const name = await exportWorkspaceZip()
			toast({ kind: 'ok', title: '已导出工作区', msg: `${name} 已开始下载` })
		} catch (e) {
			toast({ kind: 'err', title: '导出失败', msg: e instanceof Error ? e.message : String(e) })
		} finally {
			transferring = false
		}
	}

	async function importZip(file: File | undefined) {
		if (!file || transferring) return
		transferring = true
		try {
			await flushNow()
			const report = await importWorkspaceZip(file)
			toast({
				kind: 'ok',
				title: '已导入工作区',
				msg: `图 ${report.graphsTaken} 张（跳过较新 ${report.graphsSkipped}）· 库图片 ${report.libraryFiles} 张${report.groupsMerged ? ' · 分组已合并' : ''}${report.rejected ? ` · 忽略 ${report.rejected} 项` : ''}`,
			})
			onclose()
			location.reload()
		} catch (e) {
			toast({ kind: 'err', title: '导入失败', msg: e instanceof Error ? e.message : String(e) })
		} finally {
			transferring = false
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
				{:else}
					<section class="card">
						<h3>后端连接</h3>
						{#each backendRegistry.entries as entry (entry.id)}
							{@const status = statusOf(entry.status)}
							<div class="connection" data-connection-id={entry.id}>
								<div class="connection-row">
									<strong class="connection-name">{entry.name}</strong>
									<span class="ui-badge {status.tone}" title={status.label}><i class="dot"></i>{status.label}</span>
									<button type="button" class="ui-btn ghost sm" onclick={() => void connectBackend(entry.id)}>刷新 / 重连</button>
									{#if entry.kind === 'http'}
										<button type="button" class="ui-btn ghost sm" disabled={updatingConnection} onclick={() => editConnection(entry)}>编辑</button>
										<button type="button" class="ui-btn ghost sm danger" disabled={updatingConnection} onclick={() => void detach(entry.id)}>移除连接</button>
									{/if}
								</div>
								{#if connectionDraft?.id === entry.id}
									<div class="connection-edit">
										<label class="ui-field"><span>名称</span><input class="ui-input" aria-label="后端名称" bind:value={connectionDraft.name} disabled={updatingConnection} /></label>
										<label class="ui-field"><span>服务地址</span><input class="ui-input mono" aria-label="后端服务地址" bind:value={connectionDraft.baseUrl} disabled={updatingConnection} /></label>
										<div class="edit-actions"><button type="button" class="ui-btn ghost sm" disabled={updatingConnection} onclick={() => connectionDraft = null}>取消</button><button type="button" class="ui-btn primary sm" disabled={updatingConnection || !connectionDraft.name.trim() || !connectionDraft.baseUrl.trim()} onclick={() => void updateConnection()}>{updatingConnection ? '保存中…' : '保存连接'}</button></div>
									</div>
								{:else if entry.baseUrl}<p class="hint mono">{entry.baseUrl}</p>{/if}
								{#if entry.kind === 'opfs'}<p class="hint">数据存储由浏览器管理。即使获准持久存储，也不保证永久保留；清除站点数据会删除内容，建议定期导出备份。</p>{/if}
								{#if entry.error || entry.providerError}<p class="hint err">{entry.error ?? entry.providerError}</p>{/if}
								{#each entry.providers as provider (provider.id)}
									<div class="provider-tags"><span class="ui-badge provider-tag" title="Provider：{provider.id}">{provider.name}</span>{#each provider.models as model}<span class="model-tag" title={model}>{model}</span>{/each}</div>
								{/each}
								{#if entry.kind === 'http'}<p class="hint">{entry.providers.length ? '' : '暂无 Provider。'}配置与密钥由服务器管理。</p>{/if}
							</div>
						{/each}
						<div class="connection-add">
							<label class="ui-field name">
								<span>名称</span>
								<input class="ui-input" bind:value={serverName} placeholder="名称（可选）" />
							</label>
							<label class="ui-field url">
								<span>服务地址</span>
								<input class="ui-input mono" bind:value={serverUrl} placeholder="https://server.example.com" spellcheck="false" />
							</label>
							<button type="button" class="ui-btn ghost" disabled={connecting || !serverUrl.trim()} onclick={() => void addConnection()}>
								{#if connecting}<Icon name="spinner" size={12} class="spin" />{/if}添加服务器
							</button>
						</div>
					</section>
					<section class="card">
						<h3>本地存储</h3>
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
							<p class="hint">数据保存在本浏览器的 OPFS；清除站点数据会一并删除。导出备份见下方「导入 / 导出」。</p>
						{:else}
							<p class="hint">当前浏览器不支持存储配额查询。</p>
						{/if}
					</section>

					<section class="card">
						<h3>导入 / 导出</h3>
						<div class="transfer-row">
							<button type="button" class="ui-btn ghost sm" disabled={transferring} onclick={() => void exportZip()}>
								{#if transferring}<Icon name="spinner" size={12} class="spin" />{:else}<Icon name="upload" size={12} />{/if}导出 zip
							</button>
							<button type="button" class="ui-btn ghost sm" disabled={transferring} onclick={() => importInput?.click()}>
								<Icon name="move" size={12} />导入 zip
							</button>
							<input
								bind:this={importInput}
								class="ui-file"
								type="file"
								accept="application/zip,.zip"
								onchange={(e) => {
									void importZip(e.currentTarget.files?.[0])
									e.currentTarget.value = ''
								}}
							/>
						</div>
						<p class="hint">导出包含图、视图、分组、图内参考图与库图片。导入按「本地较新的图跳过」合并；Provider 配置与密钥不随 zip 转移。</p>
					</section>

					<section class="card">
						<div class="card-head">
							<h3>Local Provider</h3>
							<button type="button" class="ui-btn ghost sm" onclick={addProvider}><Icon name="plus" size={12} />新增</button>
						</div>
						<p class="hint">API Key 明文保存在本机 OPFS（可在下方随时清除），本地 Provider 可供所有后端上的图使用。</p>
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

			{#if !loading}
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
	.connection-edit { display: grid; gap: 8px; padding-top: 8px; }
	.edit-actions { display: flex; gap: 6px; justify-content: flex-end; }
	.provider-tags { display: flex; flex-wrap: wrap; align-items: center; gap: 5px; }
	.provider-tag { background: var(--ui-accent-weak); color: var(--ui-accent); }
	.model-tag { max-width: 100%; overflow-wrap: anywhere; border: 1px solid var(--ui-border-fade); border-radius: 999px; padding: 2px 7px; color: var(--ui-dim); font-size: 10px; }
	.connection {
		display: flex;
		flex-direction: column;
		gap: 5px;
		padding: 9px 0;
		border-bottom: 1px solid var(--ui-border-fade);
	}
	.connection:last-child {
		border-bottom: none;
	}
	/* 名称 + 状态徽章 + 操作按钮：同一行，控件高度一致 */
	.connection-row {
		display: flex;
		align-items: center;
		flex-wrap: wrap;
		gap: 6px;
	}
	.connection-row .ui-input {
		flex: 1 1 150px;
		min-width: 0;
		max-width: 220px;
	}
	.connection-name { flex: 1 1 150px; font-size: 12px; }
	.connection-row .ui-badge {
		flex: none;
	}
	.connection-row .ui-btn {
		flex: none;
	}
	.connection-add {
		display: flex;
		align-items: flex-end;
		flex-wrap: wrap;
		gap: 6px;
		padding-top: 10px;
	}
	.connection-add .name {
		flex: 0 1 132px;
	}
	.connection-add .url {
		flex: 1 1 190px;
	}
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
	/* 说明文字统一一档更小；路径 / 错误这类行内信息按其语义着色 */
	.card p.hint {
		font-size: 10.5px;
		color: var(--ui-faint);
	}
	.card p.hint.err {
		color: var(--ui-danger);
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
	.transfer-row {
		display: flex;
		align-items: center;
		gap: 8px;
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
