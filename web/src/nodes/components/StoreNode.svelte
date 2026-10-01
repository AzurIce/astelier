<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Icon from '../../components/Icon.svelte'
	import { rt } from '../../runtime'
	import { toast } from '../../components/toast.svelte'
	import { confirmDialog } from '../../components/confirm.svelte'
	import { removeNodeCascade } from '../actions'
	import { openLightbox } from '../../components/lightbox.svelte'
	import {
		createStore,
		deleteStore,
		deleteStoreFile,
		fetchStores,
		renameStore,
		uploadStoreFile,
		storeFileUrl,
		type StoreFileMeta,
		type StoreInfo,
	} from '../../stores'
	import { activeGraphId } from '../../graphStore'
	import type { StoreNode } from '../classes'
	
	let { data }: { data: StoreNode } = $props()

	let stores: StoreInfo[] = $state([])
	let loading = $state(false)
	let loadError: string | null = $state(null)
	let activeStore = $state('')
	let renaming: string | null = $state(null)
	let renameDraft = $state('')
	let dragOver = $state(false)
	let uploadRef: HTMLInputElement | undefined = $state()
	let gridRef: (DropCarrier & HTMLElement) | undefined = $state()

	// 每次渲染把最新 handler 挂到 DOM 节点（原生 drop 监听读取它）
	$effect(() => {
		if (!gridRef) return
		gridRef.__atelierDrop = {
			setOver: (v: boolean) => {
				dragOver = v
			},
			save: (url: string) => saveUrl(url),
		}
	})

	// 节点当前绑定的 store；变化即时保存
	$effect(() => {
		if (data.store && data.store !== activeStore) activeStore = data.store
	})

	let lastSnapshot = ''

	async function refresh(force = false) {
		const gid = activeGraphId()
		if (!gid) return
		// 只有首次/强制才显示 loading——轮询静默，避免「加载中」闪烁
		if (stores.length === 0 || force) loading = true
		loadError = null
		try {
			const next = await fetchStores(gid)
			// 内容没变就不替换 state：轮询不应触发重渲染（否则节点「时不时会闪」）
			const snapshot = JSON.stringify(next)
			const changed = snapshot !== lastSnapshot
			if (changed || force) {
				lastSnapshot = snapshot
				stores = next
			}
			// 仅在从未绑定过时兜底选第一个（建库时的显式绑定不被覆盖）
			if (!activeStore && !data.store && next.length > 0) {
				activeStore = next[0].name
				data.store = activeStore
			}
			if (activeStore && !next.some((s) => s.name === activeStore)) {
				// 绑定已不存在（被删/改名）：清空
				activeStore = ''
				data.store = ''
			}
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e)
		}
		loading = false
	}

	$effect(() => {
		if (!data.id) return
		refresh(true)
		const timer = setInterval(() => {
			if (document.hidden) return
			void refresh()
		}, 5000)
		return () => clearInterval(timer)
	})

	function active(): StoreInfo | undefined {
		return stores.find((s) => s.name === activeStore)
	}

	async function create() {
		const gid = activeGraphId()
		const base = '图库'
		let name = base
		for (let i = 2; stores.some((s) => s.name === name); i++) name = `${base}${i}`
		try {
			await createStore(gid, name)
			activeStore = name // 先绑定，再 refresh（refresh 的兜底逻辑不会再改）
			data.store = name
			await refresh(true)
			rt.area?.update('node', data.id)
		} catch (e) {
			toast({ kind: 'err', title: '创建图库失败', msg: String(e) })
		}
	}

	async function remove(s: StoreInfo) {
		const yes = await confirmDialog({
			title: `删除图库「${s.name}」`,
			message: `其中 ${s.files.length} 个文件将被永久删除，此操作不可恢复。`,
			confirmText: '删除',
			danger: true,
		})
		if (!yes) return
		try {
			await deleteStore(activeGraphId(), s.name)
			if (activeStore === s.name) {
				activeStore = ''
				data.store = ''
			}
			await refresh(true)
		} catch (e) {
			toast({ kind: 'err', title: '删除失败', msg: String(e) })
		}
	}

	function startRename(s: StoreInfo) {
		renaming = s.name
		renameDraft = s.name
	}

	async function commitRename() {
		const old = renaming
		renaming = null
		const next = renameDraft.trim()
		if (!old || !next || next === old) return
		try {
			await renameStore(activeGraphId(), old, next)
			if (activeStore === old) {
				activeStore = next
				data.store = next
			}
			await refresh(true)
		} catch (e) {
			toast({ kind: 'err', title: '重命名失败', msg: String(e) })
		}
	}

	async function pickStore(name: string) {
		activeStore = name
		data.store = name
		rt.area?.update('node', data.id)
	}

	async function upload(files: FileList | null) {
		const gid = activeGraphId()
		const s = active()
		if (!gid || !s || !files?.length) return
		for (const f of files) {
			try {
				await uploadStoreFile(gid, s.name, f)
			} catch (e) {
				toast({ kind: 'err', title: `上传 ${f.name} 失败`, msg: String(e) })
			}
		}
		await refresh(true)
	}

	async function removeFile(f: StoreFileMeta) {
		const s = active()
		if (!s) return
		try {
			await deleteStoreFile(activeGraphId(), s.name, f.name)
			await refresh(true)
		} catch (e) {
			toast({ kind: 'err', title: '删除文件失败', msg: String(e) })
		}
	}

	/** 从 store 拖出一张图：原生 H5 DnD。
	 *  img draggable + dragstart 把 URL 写进 dataTransfer，
	 *  同时置 rt.dragImage 供 pointer 兜底路径读取。 */
	function onFileDragStart(e: DragEvent, f: StoreFileMeta) {
		if (!e.dataTransfer) return
		const url = storeFileUrl(activeGraphId(), activeStore, f.name)
		e.dataTransfer.setData('text/plain', url)
		e.dataTransfer.setData('text/uri-list', url)
		e.dataTransfer.effectAllowed = 'copy'
		rt.dragImage = {
			kind: 'store',
			url,
			store: activeStore,
			file: f.name,
			w: f.w,
			h: f.h,
		}
	}
	function onFileDragEnd() {
		rt.dragImage = null
	}

	/** store 网格的放置增强：原生 DnD 监听 → 转发为组件自定义事件
 *  'storedrop'（detail = DragEvent）。绕开 Svelte 对 ondragover/ondrop
 *  的事件委托（合成 DragEvent 到元素上不触发委托处理器），
 *  handler 经由模板 on:storedrop 绑定，闭包始终最新。 */
/** store 网格放置增强：原生 DnD 监听 → 调用挂在 DOM 节点上的处理函数。
 *  handler 由模板副作用每次渲染刷新到 node.__atelierDrop，绕开 Svelte 5
 *  对 action 参数对象「变即重建」以及 ondragover/ondrop 事件委托两套坑。 */
interface DropCarrier extends HTMLElement {
	__atelierDrop?: {
		setOver: (v: boolean) => void
		save: (url: string) => void | Promise<void>
	}
}
function storeDropTarget(node: DropCarrier): { destroy: () => void } {
	const enter = (e: DragEvent) => {
		e.preventDefault()
		node.__atelierDrop?.setOver(true)
	}
	const over = (e: DragEvent) => {
		e.preventDefault()
		node.__atelierDrop?.setOver(true)
	}
	const leave = (e: DragEvent) => {
		e.preventDefault()
		node.__atelierDrop?.setOver(false)
	}
	const drop = (e: DragEvent) => {
		e.preventDefault()
		node.__atelierDrop?.setOver(false)
		const url = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain') || ''
		if (url) void node.__atelierDrop?.save(url)
	}
	node.addEventListener('dragenter', enter)
	node.addEventListener('dragover', over)
	node.addEventListener('dragleave', leave)
	node.addEventListener('drop', drop)
	return {
		destroy() {
			node.removeEventListener('dragenter', enter)
			node.removeEventListener('dragover', over)
			node.removeEventListener('dragleave', leave)
			node.removeEventListener('drop', drop)
			delete node.__atelierDrop
		},
	}
}

/** 把一张图片 URL 保存进当前图库（拖入的落地动作） */
async function saveUrl(url: string): Promise<void> {
	const s = active()
	if (!s) {
		toast({ kind: 'info', title: '请先选择或创建图库' })
		return
	}
	try {
		const res = await fetch(url)
		if (!res.ok) throw new Error(`HTTP ${res.status}`)
		const blob = await res.blob()
		const name = decodeURIComponent(url.split('/').pop() || 'image.png')
		await uploadStoreFile(activeGraphId(), s.name, new File([blob], name, { type: blob.type || 'image/png' }))
		await refresh(true)
		toast({ kind: 'ok', title: `已保存到「${s.name}」`, msg: name })
	} catch (err) {
		toast({ kind: 'err', title: '保存失败', msg: err instanceof Error ? err.message : String(err) })
	}
}
</script>

<NodeFrame
	nodeId={data.id}
	type="store"
	icon="layers"
	name="Image Store"
	desc={activeStore ? `${activeStore} · ${active()?.files.length ?? 0} 张` : '未绑定图库'}
	selected={data.selected}
	ondelete={() => void removeNodeCascade(data.id)}
>
	{#snippet body()}
		{#if loadError}
			<div class="ui-error">
				<Icon name="alert" size={13} />
				<span>{loadError}</span>
			</div>
		{/if}
		{#if loading}
			<div class="ui-empty-hint">加载中…</div>
		{/if}

		<!-- 图库选择 + 管理 -->
		<div class="store-tabs">
			{#each stores as s (s.name)}
				{#if renaming === s.name}
					<input
						class="rename"
						bind:value={renameDraft}
						onkeydown={(e) => {
							if (e.key === 'Enter') void commitRename()
							if (e.key === 'Escape') renaming = null
						}}
						onblur={() => void commitRename()}
					/>
				{:else}
					<button
						type="button"
						class="store-tab"
						class:active={s.name === activeStore}
						onclick={() => void pickStore(s.name)}
						ondblclick={() => startRename(s)}
						title={`${s.name}（${s.files.length} 张）· 双击重命名`}
					>
						<Icon name="folder" size={13} />
						<span class="name">{s.name}</span>
						<span class="count mono">{s.files.length}</span>
					</button>
				{/if}
			{/each}
			<button type="button" class="store-tab add" onclick={() => void create()} title="新建图库">
				<Icon name="plus" size={13} />
			</button>
			{#if active()}
				<button type="button" class="store-tab del" onclick={() => void remove(active()!)} title="删除当前图库">
					<Icon name="trash" size={13} />
				</button>
			{/if}
		</div>

		<!-- 图片网格（点击预览 + 拖出） -->
		<div
			bind:this={gridRef}
			class="store-grid"
			class:empty={!active() || active()!.files.length === 0}
			class:dragover={dragOver}
			role="listbox"
			tabindex="0"
			aria-label="图库图片"
			use:storeDropTarget
		>
			{#if !active()}
				<div class="store-empty">
					<span class="empty-icon"><Icon name="layers" size={18} /></span>
					<strong>还没有图库</strong>
					<span class="empty-sub">图库用来显式收藏生成结果<br />可建多个、随时切换</span>
					<button type="button" class="ui-btn primary sm" onclick={() => void create()}>
						<Icon name="plus" size={12} />
						新建图库
					</button>
				</div>
			{:else if active()!.files.length === 0}
				<div class="store-empty">
					<span class="empty-icon"><Icon name="upload" size={18} /></span>
					<strong>「{activeStore}」还是空的</strong>
					<span class="empty-sub">把 Generate / Preview 的结果图拖进来保存<br />或直接上传本地图片</span>
					<button type="button" class="ui-btn ghost sm" onclick={() => uploadRef?.click()}>
						<Icon name="upload" size={12} />
						上传图片
					</button>
				</div>
			{:else}
				{#each active()!.files as f (f.name)}
					<div
						class="cell"
						role="option"
						aria-selected="false"
						title={`${f.name}（${f.w ?? '?'}×${f.h ?? '?'}）· 拖出到画布 / 单击预览`}
					>
						<button
							type="button"
							class="thumb"
							draggable="true"
							ondragstart={(e) => onFileDragStart(e, f)}
							ondragend={onFileDragEnd}
							onclick={() => openLightbox(storeFileUrl(activeGraphId(), activeStore, f.name))}
							title={`${f.name} · 拖出使用 / 单击预览`}
						>
							<img src={storeFileUrl(activeGraphId(), activeStore, f.name)} alt={f.name} draggable="false" />
						</button>
						<button
							type="button"
							class="del"
							aria-label="删除"
							title="删除"
							onclick={() => void removeFile(f)}
						>
							<Icon name="x" size={11} />
						</button>
					</div>
				{/each}
			{/if}
		</div>

		<div class="store-foot">
			<span class="hint">{active() ? `拖出图片到画布使用 · 拖入结果图保存` : ''}</span>
			<button type="button" class="ui-btn ghost sm" onclick={() => uploadRef?.click()}>
				<Icon name="upload" size={12} />
				上传
			</button>
			<input
				bind:this={uploadRef}
				type="file"
				accept="image/*"
				multiple
				style="display:none"
				onchange={(e) => {
					void upload(e.currentTarget.files)
					e.currentTarget.value = ''
				}}
			/>
		</div>
	{/snippet}
</NodeFrame>

<style>
	.store-tabs {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
	}
	.store-tab {
		display: inline-flex;
		align-items: center;
		gap: 5px;
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--ui-border-fade);
		border-radius: 999px;
		background: var(--ui-input);
		color: var(--ui-dim);
		font: inherit;
		font-size: 11px;
		cursor: pointer;
		transition: all var(--ui-fast);
	}
	.store-tab:hover {
		color: var(--ui-text);
		border-color: var(--ui-dim);
	}
	.store-tab.active {
		background: var(--ui-accent-weak);
		border-color: var(--ui-accent);
		color: var(--ui-text);
	}
	.store-tab.add,
	.store-tab.del {
		padding: 0 6px;
	}
	.store-tab .count {
		color: var(--ui-faint);
		font-size: 10px;
	}
	.store-tab.active .count {
		color: var(--ui-accent);
	}
	.rename {
		height: 24px;
		padding: 0 8px;
		border: 1px solid var(--ui-accent);
		border-radius: 999px;
		background: var(--ui-input);
		color: var(--ui-text);
		font: inherit;
		font-size: 11px;
		outline: none;
	}

	.store-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 6px;
		min-height: 88px;
		padding: 6px;
		border: 1px dashed var(--ui-border-fade);
		border-radius: var(--ui-r-control);
		background: var(--ui-input);
		transition: border-color var(--ui-fast), background var(--ui-fast);
	}
	.store-grid:has(.store-empty) {
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.store-empty {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 4px;
		padding: 10px 8px;
		text-align: center;
	}
	.store-empty .empty-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 36px;
		height: 36px;
		margin-bottom: 2px;
		border-radius: 50%;
		background: var(--ui-track);
		color: var(--ui-dim);
	}
	.store-empty strong {
		font-size: 12px;
		font-weight: 600;
		color: var(--ui-text);
	}
	.store-empty .empty-sub {
		color: var(--ui-faint);
		font-size: 10.5px;
		line-height: 1.6;
		margin-bottom: 4px;
	}
	.store-empty :global(.ui-btn) {
		margin-top: 2px;
	}
	.cell {
		position: relative;
		aspect-ratio: 1;
		border-radius: var(--ui-r-control);
		overflow: hidden;
		background: var(--ui-track);
		cursor: grab;
	}
	.cell .thumb {
		display: block;
		width: 100%;
		height: 100%;
		padding: 0;
		border: none;
		background: none;
		cursor: grab;
	}
	.cell .thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.cell .del {
		position: absolute;
		top: 3px;
		right: 3px;
		width: 18px;
		height: 18px;
		display: flex;
		align-items: center;
		justify-content: center;
		border: none;
		border-radius: 5px;
		background: rgba(0, 0, 0, 0.55);
		color: #fff;
		cursor: pointer;
		opacity: 0;
		transition: opacity var(--ui-fast);
	}
	.cell:hover .del {
		opacity: 1;
	}
	.store-foot {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 8px;
	}
	.store-foot .hint {
		color: var(--ui-faint);
		font-size: 10px;
	}
</style>
