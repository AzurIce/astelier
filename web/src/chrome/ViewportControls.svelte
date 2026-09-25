<script lang="ts">
	import { onMount } from 'svelte'
	import IconButton from '../components/IconButton.svelte'
	import { currentZoom, fitView, resetZoom, subscribeZoom, zoomBy } from '../editor'

	// 右下悬浮视口控件：缩放百分比（点击复位 100%）
	let zoom = $state(currentZoom())

	onMount(() => subscribeZoom((k) => (zoom = k)))
</script>

<div class="viewport-controls">
	<IconButton icon="zoomOut" label="缩小" sm onclick={() => zoomBy(1 / 1.25)} />
	<button
		type="button"
		class="zoom-label"
		title="回到 100%"
		onclick={() => resetZoom()}
	>
		{Math.round(zoom * 100)}%
	</button>
	<IconButton icon="zoomIn" label="放大" sm onclick={() => zoomBy(1.25)} />
	<IconButton icon="fit" label="适应视图" sm onclick={() => fitView()} />
</div>
