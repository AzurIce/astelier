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

	/** 图内 store URL：图片随图走，UI 不可见（内联感知） */
	const refUrl = $derived(data.refFile ? graphStoreUrl(gid, data.refFile) : '')

	/** 上传 / 外部拖入 → 复制进图 store，节点只留 file 引用 */
	async function ingest(src: File | Blob | undefined | null, name: string) {
		if (!gid) {
			error = '图尚未加载'
			return
		}
		if (!src) return
		busy = true
		error = null
		try {
			const m = await uploadGraphStoreFile(gid, name, src)
			editNode<LoadImageNode>(data.id, (n) => {
				n.refFile = m.name
				n.fileName = m.name
				n.w = m.w
				n.h = m.h
			})
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
			await ingest(blob, decodeURIComponent(url.split('/').pop() || 'image.png'))
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			toast({ kind: 'err', title: '取图失败', msg: error })
		}
		busy = false
	}

	function clear() {
		// 只解除引用；文件由节点删除时的引用扫描统一回收
		editNode<LoadImageNode>(data.id, (n) => {
			n.refFile = null
			n.fileName = ''
			n.w = undefined
			n.h = undefined
		})
	}

	const title = $derived(
		busy ? '读取中…' : data.refFile ? `${data.fileName} · 图内图片` : '参考图 / 垫图',
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
				const f = e.dataTransfer?.files?.[0]
				if (f) void ingest(f, f.name)
				// 没有文件：可能是从画布别的节点拖来的图片，交给 acceptImageDrop
			}}
		>
			<input
				type="file"
				accept="image/*"
				use:noNodeDrag
				onchange={(e) => {
					const f = e.currentTarget.files?.[0]
					if (f) void ingest(f, f.name)
				}}
			/>
			{#if busy}
				<Icon name="spinner" size={18} class="spin" />
				<span>读取中…</span>
			{:else}
				<Icon name="upload" size={18} />
				<span class="file-name">{data.fileName || '上传 / 从库拖入'}</span>
			{/if}
		</label>

		{#if error}
			<div class="ui-error">
				<Icon name="alert" size={13} />
				<span>{error}</span>
			</div>
		{:else if refUrl}
			<div class="img-wrap">
				<button
					type="button"
					class="img-btn"
					title="点击查看大图"
					use:noNodeDrag
					onclick={() => openLightbox(refUrl)}
				>
					<img class="ui-img thumbnail" src={refUrl} alt={data.fileName} />
				</button>
				<IconButton icon="trash" label="移除图片" sm onclick={clear} />
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
