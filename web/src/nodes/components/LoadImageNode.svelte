<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { scheduleSave } from '../../persist'
	import type { LoadImageNode } from '../classes'
	import { uploadAsset } from '../../api'

	export let data: LoadImageNode
	export let emit: (p: AreaExtra) => void

	let busy = false
	let error: string | null = null

	async function pick(ev: Event) {
		const input = ev.currentTarget as HTMLInputElement
		const file = input.files?.[0]
		if (!file) return
		busy = true
		error = null
		try {
			data.assetUrl = await uploadAsset(file)
			data.fileName = file.name
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		}
		busy = false
		input.value = ''
		rt.area?.update('node', data.id)
		scheduleSave()
	}
</script>

<div class="an-node" class:selected={data.selected} data-node-id={data.id}>
	<div class="an-title an-t-image">Image</div>
	<div class="an-body">
		<label class="an-file">
			<input type="file" accept="image/*" on:pointerdown|stopPropagation on:change={pick} />
			{busy ? '上传中…' : data.fileName || '选择图片…'}
		</label>
		{#if error}
			<div class="an-error">{error}</div>
		{:else if data.assetUrl}
			<img class="an-preview" src={data.assetUrl} alt={data.fileName} />
		{/if}
	</div>
	<div class="an-out">
		<span class="an-port-label">image</span>
		<Ref
			class="an-socket an-sock-image"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'output',
						key: 'image',
						nodeId: data.id,
						element,
						payload: data.outputs.image!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
	</div>
</div>
