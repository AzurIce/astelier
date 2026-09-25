<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import type { PreviewNode } from '../classes'

	export let data: PreviewNode
	export let emit: (p: AreaExtra) => void
</script>

<div class="an-node" class:selected={data.selected} data-node-id={data.id}>
	<div class="an-title an-t-image">Preview</div>
	<div class="an-in">
		<Ref
			class="an-socket an-sock-image"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'image',
						nodeId: data.id,
						element,
						payload: data.inputs.image!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">image</span>
	</div>
	<div class="an-body">
		{#if data.displayUrl}
			<img class="an-preview" src={data.displayUrl} alt="预览" />
		{:else}
			<div class="an-empty">连入 image 后 Run</div>
		{/if}
	</div>
</div>
