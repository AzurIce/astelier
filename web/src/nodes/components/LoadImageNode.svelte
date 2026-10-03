<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import IconButton from '../../components/IconButton.svelte'
	import Icon from '../../components/Icon.svelte'
	import type { AreaExtra } from '../types'
	import { toast } from '../../components/toast.svelte'
	import { openLightbox } from '../../components/lightbox.svelte'
	import { editNode, removeNodeCascade } from '../actions'
	import { noNodeDrag } from '../noNodeDrag'
	import { acceptImageDrop } from '../acceptImageDrop'
	import { activeGraphId } from '../../graphStore'
	import { uploadGraphStoreFile, graphStoreUrl } from '../../graphStore'
	import type { LoadImageNode } from '../classes'

	let { data, emit }: { data: LoadImageNode; emit: (p: AreaExtra) => void } = $props()

	let busy = $state(false)
	let over = $state(false)
	let error: string | null = $state(null)

	const gid = activeGraphId()

	/** 节点内引用 → 图内 store URL（顺序即 image[] 顺序） */
	const urls = $derived((data.images ?? []).map((i) => graphStoreUrl(gid, i.file)))

	/** 上传 / 外部拖入 → 批量复制进图 store，节点只留 file 引用 */
	async function ingest(files: (File | Blob | null | undefined)[], nameOf?: (f: File | Blob) => string) {
		if (!gid) {
			error = '图尚未加载'
			return
		}
		const list = files.filter((f): f is File | Blob => !!f)
		if (!list.length) return
		busy = true
		error = null
		let ok = 0
		let skipped = 0
		try {
			for (const f of list) {
				const name = nameOf ? nameOf(f) : (f as File).name || 'image.png'
				const m = await uploadGraphStoreFile(gid, name, f)
				// 同名去重在组件这侧先判，避免为重复图重复写盘
				if (data.images.some((i) => i.file === m.name)) {
					skipped++
					continue
				}
				editNode<LoadImageNode>(data.id, (n) =>
					n.addImage({ file: m.name, name: m.name, w: m.w, h: m.h })
				)
				ok++
			}
			if (skipped > 0 && ok === 0) {
				toast({ kind: 'ok', title: '图片已在节点内', msg: `跳过 ${skipped} 张重复图片` })
			} else if (skipped > 0) {
				toast({ kind: 'ok', title: '部分图片已在节点内', msg: `新增 ${ok} 张，跳过 ${skipped} 张重复` })
			}
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			toast({ kind: 'err', title: '图片入库失败', msg: error })
		}
		busy = false
	}

	/** 从底部库拖入：fetch 图片 → 复制进图 store（图自包含，删图不动库） */
	async function ingestFromUrl(url: string) {
		busy = true
		error = null
		try {
			const res = await fetch(url)
			if (!res.ok) throw new Error(`HTTP ${res.status}`)
			const blob = await res.blob()
			await ingest([blob], () => decodeURIComponent(url.split('/').pop() || 'image.png'))
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			toast({ kind: 'err', title: '取图失败', msg: error })
		}
		busy = false
	}

	function remove(file: string) {
		// 只解除引用；文件由节点删除时的引用扫描统一回收
		editNode<LoadImageNode>(data.id, (n) => n.removeImage(file))
	}

	function clearAll() {
		editNode<LoadImageNode>(data.id, (n) => {
			n.images = []
		})
	}

	const title = $derived(
		busy
			? '读取中…'
			: data.images.length > 1
				? `${data.images.length} 张图内图片`
				: data.images.length === 1
					? `${data.images[0].name} · 图内图片`
					: '参考图 / 垫图',
	)
</script>

<NodeFrame
	nodeId={data.id}
	type="image"
	icon="image"
	name="Image"
	{title}
	selected={data.selected}
	ondelete={() => void removeNodeCascade(data.id)}
>
	{#snippet body()}
		<label
			class="ui-drop"
			class:over
			use:acceptImageDrop={{ onDrop: (img) => void ingestFromUrl(img.url) }}
			ondragover={(e) => {
				e.preventDefault()
				over = true
			}}
			ondragleave={() => (over = false)}
			ondrop={(e) => {
				e.preventDefault()
				over = false
				const files = Array.from(e.dataTransfer?.files ?? [])
				if (files.length) void ingest(files)
				// 没有文件：可能是从画布别的节点拖来的图片，交给 acceptImageDrop
			}}
		>
			<input
				type="file"
				accept="image/*"
				multiple
				use:noNodeDrag
				onchange={(e) => {
					const files = Array.from(e.currentTarget.files ?? [])
					if (files.length) void ingest(files)
					e.currentTarget.value = ''
				}}
			/>
			{#if busy}
				<Icon name="spinner" size={18} class="spin" />
				<span>读取中…</span>
			{:else}
				<Icon name="upload" size={18} />
				<span class="file-name">
					{data.images.length > 0 ? `${data.images.length} 张 · 继续添加` : '上传 / 从库拖入'}
				</span>
			{/if}
		</label>

		{#if error}
			<div class="ui-error">
				<Icon name="alert" size={13} />
				<span>{error}</span>
			</div>
		{:else if urls.length}
			<div class="img-grid">
				{#each data.images as img, i (img.file)}
					<div class="img-wrap">
						<button
							type="button"
							class="img-btn"
							title={img.name}
							use:noNodeDrag
							onclick={() => openLightbox(urls[i])}
						>
							<img class="ui-img thumbnail" src={urls[i]} alt={img.name} />
						</button>
						<IconButton icon="trash" label={`移除 ${img.name}`} sm onclick={() => remove(img.file)} />
					</div>
				{/each}
			</div>
			{#if data.images.length > 1}
				<div class="hint">按添加顺序发送，最多 16 张</div>
				<button type="button" class="ui-btn ghost sm clear-all" use:noNodeDrag onclick={clearAll}>
					清空全部
				</button>
			{/if}
		{/if}
	{/snippet}

	{#snippet outputs()}
		<Port {data} {emit} side="output" port="image" label="image" tone="image" />
	{/snippet}
</NodeFrame>

<style>
	.img-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 6px;
	}
	/* 多图并排：单张预览可以大，网格里统一收小，节点别被撑爆 */
	.img-grid :global(.ui-img) {
		max-height: 92px;
	}
	.img-wrap {
		position: relative;
		display: flex;
		justify-content: center;
		min-width: 0;
	}
	.img-wrap :global(.ui-btn.icon) {
		position: absolute;
		right: 0;
		top: 0;
		background: var(--ui-panel);
	}	.clear-all {
		margin-top: 6px;
		width: 100%;
	}
	.hint {
		margin-top: 6px;
		color: var(--ui-dim);
		font-size: 10.5px;
		text-align: center;
	}
</style>
