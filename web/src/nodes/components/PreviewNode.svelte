<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import Icon from '../../components/Icon.svelte'
	import { openLightbox } from '../../components/lightbox.svelte'
	import { removeNodeCascade } from '../actions'
	import type { AreaExtra } from '../types'
	import type { PreviewNode } from '../classes'

	let { data, emit }: { data: PreviewNode; emit: (p: AreaExtra) => void } = $props()
</script>

<NodeFrame
	nodeId={data.id}
	type="preview"
	icon="preview"
	name="Preview"
	desc="结果展示"
	selected={data.selected}
	ondelete={() => removeNodeCascade(data.id)}
>
	{#snippet inputs()}
		<Port {data} {emit} side="input" port="image" label="image" tone="image" />
	{/snippet}

	{#snippet body()}
		{#if data.displayUrl}
			<button
				type="button"
				class="img-btn draggable-img"
				title="点击查看大图 · 拖入底部「库」收藏"
				draggable="true"
				ondragstart={(e) => {
					if (!e.dataTransfer || !data.displayUrl) return
					e.dataTransfer.setData('text/plain', data.displayUrl)
					e.dataTransfer.setData('text/uri-list', data.displayUrl)
					e.dataTransfer.effectAllowed = 'copy'
				}}
				onclick={() => data.displayUrl && openLightbox(data.displayUrl)}
			>
				<img class="ui-img thumbnail" src={data.displayUrl} alt="预览" draggable="false" />
			</button>
		{:else}
			<div class="ui-empty-hint">
				<Icon name="image" size={22} />
				<span>连入 image 输出<br />点 Run 查看结果</span>
			</div>
		{/if}
	{/snippet}
</NodeFrame>
