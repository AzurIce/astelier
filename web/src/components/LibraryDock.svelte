<script lang="ts">
	import { onMount, type Snippet } from 'svelte'
	import Icon from '../components/Icon.svelte'
	import IconButton from '../components/IconButton.svelte'
	import { toast } from '../components/toast.svelte'
	import { openLightbox } from '../components/lightbox.svelte'
	import { deleteStoreFile, fetchStore, storeUrl, uploadStoreFile, type StoreFileMeta } from '../store'

	// 底部「库」面板：全局收藏（data/stores/，平铺）。
	// - 缩略图网格：单击预览、拖出到画布（H5 DnD）
	// - 上传按钮 / 拖文件进来的 drop 区
	// - 结果图拖进来 = 收藏（Generate / Preview 的图可拖）
	// - 单张删除；空态引导
	let {
		collapsed = false,
		ontoggle,
		children,
	}: { collapsed?: boolean; ontoggle?: () => void; children?: Snippet } = $props()

	let files: StoreFileMeta[] = $state([])
	let loading = $state(false)
	let dragOver = $state(false)
	let uploadRef: HTMLInputElement | undefined = $state()

	async function refresh() {
		loading = true
		try {
			files = await fetchStore()
		} catch (e) {
			toast({ kind: 'err', title: '读取库失败', msg: e instanceof Error ? e.message : String(e) })
		}
		loading = false
	}

	onMount(refresh)

	async function upload(list: FileList | null) {
		if (!list?.length) return
		for (const f of list) {
			try {
				await uploadStoreFile(f)
			} catch (e) {
				toast({ kind: 'err', title: `上传 ${f.name} 失败`, msg: e instanceof Error ? e.message : String(e) })
			}
		}
		await refresh()
	}

	async function remove(f: StoreFileMeta) {
		try {
			await deleteStoreFile(f.name)
			await refresh()
		} catch (e) {
			toast({ kind: 'err', title: '删除失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	/** 结果图 / 外部图片 URL 拖进来 → 收藏 */
	async function collect(url: string) {
		try {
			const res = await fetch(url)
			if (!res.ok) throw new Error(`HTTP ${res.status}`)
			const blob = await res.blob()
			const name = decodeURIComponent(url.split('/').pop() || `image-${Date.now()}.png`)
			await uploadStoreFile(new File([blob], name, { type: blob.type || 'image/png' }))
			await refresh()
			toast({ kind: 'ok', title: `已收入库`, msg: name })
		} catch (e) {
			toast({ kind: 'err', title: '收藏失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	function onDragStart(e: DragEvent, f: StoreFileMeta) {
		if (!e.dataTransfer) return
		const url = storeUrl(f.name)
		e.dataTransfer.setData('text/plain', url)
		e.dataTransfer.setData('text/uri-list', url)
		e.dataTransfer.effectAllowed = 'copy'
	}
</script>

{#if collapsed}
	<button type="button" class="dock-rail" onclick={() => ontoggle?.()} title="展开「库」面板">
		<Icon name="layers" size={14} />
		<span>库</span>
		<Icon name="chevronDown" size={12} />
	</button>
{:else}
	<section class="dock" aria-label="库">
		<header class="dock-head">
			<Icon name="layers" size={14} />
			<strong>库</strong>
			<span class="count mono">{files.length}</span>
			<span class="grow"></span>
			{#if children}
				{@render children()}
			{/if}
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
			<IconButton icon="chevronDown" label="收起面板" sm onclick={() => ontoggle?.()} />
		</header>

		<div
			class="dock-body"
			class:dragover={dragOver}
			role="list"
			aria-label="库中的图片"
			ondragover={(e) => {
				e.preventDefault()
				dragOver = true
			}}
			ondragleave={() => (dragOver = false)}
			ondrop={(e) => {
				e.preventDefault()
				dragOver = false
				const url = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain')
				if (url) void collect(url)
			}}
		>
			{#if files.length === 0 && !loading}
				<div class="dock-empty">
					<Icon name="layers" size={22} />
					<span>库还是空的</span>
					<small>跑出的满意结果，拖到这里收藏；也可以直接上传本地图片</small>
				</div>
			{:else}
				{#each files as f (f.name)}
					<figure class="card" title={`${f.name} · ${f.w ?? '?'}×${f.h ?? '?'}`}>
						<button
							type="button"
							class="thumb"
							draggable="true"
							ondragstart={(e) => onDragStart(e, f)}
							onclick={() => openLightbox(storeUrl(f.name))}
						>
							<img src={storeUrl(f.name)} alt={f.name} loading="lazy" draggable="false" />
						</button>
						<figcaption>
							<span class="name" title={f.name}>{f.name}</span>
							<IconButton
								icon="trash"
								label="从库中删除"
								sm
								onclick={() => void remove(f)}
							/>
						</figcaption>
					</figure>
				{/each}
			{/if}
		</div>
	</section>
{/if}

<style>
	/* ---- 收起态：一条细轨 ---- */
	.dock-rail {
		position: fixed;
		left: 50%;
		bottom: 0;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 6px;
		height: 22px;
		padding: 0 14px;
		border: none;
		border-radius: var(--ui-r-menu) var(--ui-r-menu) 0 0;
		background: var(--ui-panel);
		border: 1px solid var(--ui-border-fade);
		border-bottom: none;
		color: var(--ui-dim);
		font: inherit;
		font-size: 11px;
		cursor: pointer;
		z-index: 40;
	}
	.dock-rail:hover {
		color: var(--ui-text);
	}
	.dock-rail :global(.ui-icon:first-child) {
		color: var(--ui-accent);
	}

	/* ---- 展开态 ---- */
	.dock {
		flex: none;
		height: 208px;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--ui-border-fade);
		background: var(--ui-panel);
	}
	.dock-head {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 36px;
		padding: 0 12px;
		flex: none;
		border-bottom: 1px solid var(--ui-border-fade);
	}
	.dock-head :global(.ui-icon:first-child) {
		color: var(--ui-accent);
	}
	.dock-head strong {
		font-size: 12px;
		font-weight: 650;
	}
	.dock-head .count {
		padding: 1px 7px;
		border-radius: 999px;
		background: var(--ui-input);
		color: var(--ui-dim);
		font-size: 10px;
	}
	.dock-head .grow {
		flex: 1;
	}

	.dock-body {
		flex: 1;
		min-height: 0;
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 10px 12px;
		overflow-x: auto;
		overflow-y: hidden;
		transition: background var(--ui-fast), box-shadow var(--ui-fast);
	}
	.dock-body.dragover {
		background: var(--ui-accent-weak);
		box-shadow: inset 0 0 0 2px var(--ui-accent);
	}
	.dock-empty {
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 5px;
		color: var(--ui-faint);
		text-align: center;
	}
	.dock-empty span {
		font-size: 12px;
		font-weight: 600;
		color: var(--ui-dim);
	}
	.dock-empty small {
		font-size: 10.5px;
	}

	.card {
		flex: none;
		width: 128px;
		margin: 0;
		border-radius: var(--ui-r-control);
		background: var(--ui-card);
		border: 1px solid var(--ui-border-fade);
		overflow: hidden;
		transition: border-color var(--ui-fast), transform var(--ui-fast) var(--ui-ease);
	}
	.card:hover {
		border-color: var(--ui-accent);
		transform: translateY(-2px);
	}
	.card .thumb {
		display: block;
		width: 100%;
		height: 104px;
		padding: 0;
		border: none;
		background: var(--ui-input);
		cursor: grab;
	}
	.card .thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}
	.card figcaption {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 4px 6px;
	}
	.card figcaption .name {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-size: 10.5px;
		color: var(--ui-dim);
	}
	.card figcaption :global(.ui-btn.icon) {
		opacity: 0;
	}
	.card:hover figcaption :global(.ui-btn.icon) {
		opacity: 1;
	}
</style>
