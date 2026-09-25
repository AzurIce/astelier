<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import IconButton from '../../components/IconButton.svelte'
	import Icon from '../../components/Icon.svelte'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { scheduleSave } from '../../graphStore'
	import { uploadAsset } from '../../api'
	import { toast } from '../../components/toast.svelte'
	import { openLightbox } from '../../components/lightbox.svelte'
	import { removeNodeCascade } from '../actions'
	import type { LoadImageNode } from '../classes'

	let { data, emit }: { data: LoadImageNode; emit: (p: AreaExtra) => void } = $props()

	let busy = $state(false)
	let over = $state(false)
	let error: string | null = $state(null)

	function touch() {
		rt.area?.update('node', data.id)
		scheduleSave()
	}

	async function pick(file: File | undefined | null) {
		if (!file) return
		busy = true
		error = null
		try {
			data.assetUrl = await uploadAsset(file)
			data.fileName = file.name
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			toast({ kind: 'err', title: '图片上传失败', msg: error })
		}
		busy = false
		touch()
	}
</script>

<NodeFrame
	nodeId={data.id}
	type="image"
	icon="image"
	name="Image"
	desc={busy ? '上传中…' : data.fileName || '参考图 / 垫图'}
	selected={data.selected}
	ondelete={() => removeNodeCascade(data.id)}
>
	{#snippet body()}
		<label
			class="ui-drop"
			class:over
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
				onpointerdown={(e) => e.stopPropagation()}
				onchange={(e) => void pick(e.currentTarget.files?.[0])}
			/>
			{#if busy}
				<Icon name="spinner" size={18} class="spin" />
				<span>上传中…</span>
			{:else}
				<Icon name="upload" size={18} />
				<span class="file-name">{data.fileName || '选择或拖入图片'}</span>
			{/if}
		</label>

		{#if error}
			<div class="ui-error">
				<Icon name="alert" size={13} />
				<span>{error}</span>
			</div>
		{:else if data.assetUrl}
			<div class="img-wrap">
				<button
					type="button"
					class="img-btn"
					title="点击查看大图"
					onpointerdown={(e) => e.stopPropagation()}
					onclick={() => data.assetUrl && openLightbox(data.assetUrl)}
				>
					<img class="ui-img thumbnail" src={data.assetUrl} alt={data.fileName} />
				</button>
				<IconButton
					icon="trash"
					label="移除图片"
					sm
					onclick={() => {
						data.assetUrl = null
						data.fileName = ''
						touch()
					}}
				/>
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
