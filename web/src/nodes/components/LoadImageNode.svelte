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
	import { inlineDel, inlineGet, inlineKey, inlinePut } from '../../inlineImages'
	import { blobToDataUrl } from '../../inlineImages'
	import { activeGraphId } from '../../graphStore'
	import type { LoadImageNode } from '../classes'

	let { data, emit }: { data: LoadImageNode; emit: (p: AreaExtra) => void } = $props()

	let busy = $state(false)
	let over = $state(false)
	let error: string | null = $state(null)

	const gid = activeGraphId()

	/** 上传 = inline（IndexedDB，不落服务端） */
	async function pick(file: File | undefined | null) {
		if (!file) return
		busy = true
		error = null
		try {
			const dataUrl = await blobToDataUrl(file)
			await inlinePut(inlineKey(gid, data.id), file)
			editNode<LoadImageNode>(data.id, (n) => {
				n.assetUrl = dataUrl // inline：data URL 即数据本体
				n.fileName = file.name
				n.inline = true
				n.ref = null
			})
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			toast({ kind: 'err', title: '读取图片失败', msg: error })
		}
		busy = false
	}

	/** 从 image store 拖入 = 引用（/gstore/…） */
	async function acceptStore(url: string, file: string, w?: number, h?: number) {
		await inlineDel(inlineKey(gid, data.id))
		editNode<LoadImageNode>(data.id, (n) => {
			n.assetUrl = url
			n.fileName = file
			n.inline = false
			n.ref = { store: decodeURIComponent(url.split('/')[3] ?? ''), file: decodeURIComponent(file) }
			n.w = w
			n.h = h
		})
		toast({ kind: 'ok', title: '已引用图库图片', msg: decodeURIComponent(file) })
	}

	async function clear() {
		await inlineDel(inlineKey(gid, data.id))
		editNode<LoadImageNode>(data.id, (n) => {
			n.assetUrl = null
			n.fileName = ''
			n.inline = false
			n.ref = null
		})
	}

	const title = $derived(
		busy
			? '读取中…'
			: data.inline
				? `${data.fileName} · 节点内联`
				: data.ref
					? `${data.fileName} · 图库引用`
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
			use:acceptImageDrop={{
				onDrop: (img) => acceptStore(img.url, img.file, img.w, img.h),
			}}
			ondragover={(e) => {
				e.preventDefault()
				over = true
			}}
			ondragleave={() => (over = false)}
			ondrop={(e) => {
				e.preventDefault()
				over = false
				void pick(e.dataTransfer?.files?.[0])
			}}
		>
			<input
				type="file"
				accept="image/*"
				use:noNodeDrag
				onchange={(e) => void pick((e.target as HTMLInputElement).files?.[0])}
			/>
			{#if busy}
				<Icon name="spinner" size={18} class="spin" />
				<span>读取中…</span>
			{:else}
				<Icon name="upload" size={18} />
				<span class="file-name">{data.fileName || '上传 / 从图库拖入'}</span>
			{/if}
		</label>

		{#if error}
			<div class="ui-error">
				<Icon name="alert" size={13} />
				<span>{error}</span>
			</div>
		{:else if data.inline}
			{#await inlineGet(inlineKey(gid, data.id)) then blob}
				{#if blob}
					<div class="img-wrap">
						<button
							type="button"
							class="img-btn"
							title="点击查看大图"
							use:noNodeDrag
							onclick={() => openLightbox(URL.createObjectURL(blob))}
						>
							<img class="ui-img thumbnail" src={URL.createObjectURL(blob)} alt={data.fileName} />
						</button>
						<IconButton icon="trash" label="移除图片" sm onclick={() => void clear()} />
					</div>
				{/if}
			{/await}
		{:else if data.assetUrl}
			<div class="img-wrap">
				<button
					type="button"
					class="img-btn"
					title="点击查看大图"
					use:noNodeDrag
					onclick={() => openLightbox(data.assetUrl!)}
				>
					<img class="ui-img thumbnail" src={data.assetUrl} alt={data.fileName} />
				</button>
				<IconButton icon="trash" label="移除图片" sm onclick={() => void clear()} />
			</div>
		{/if}
	{/snippet}

	{#snippet outputs()}
		<Port {data} {emit} side="output" port="image" label="image" tone="image" />
	{/snippet}
</NodeFrame>

<style>
	.img-wrap {
		position: relative;
		display: flex;
		justify-content: center;
	}
	.img-wrap :global(.ui-btn.icon) {
		position: absolute;
		right: 0;
		top: 0;
		background: var(--ui-panel);
	}
</style>
