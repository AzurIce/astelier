<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import type { GenerateNode } from '../classes'

	export let data: GenerateNode
	export let emit: (p: AreaExtra) => void

	function touch() {
		rt.area?.update('node', data.id)
	}
</script>

<div class="an-node" class:selected={data.selected}>
	<div class="an-title an-t-model">Generate</div>

	<div class="an-in">
		<Ref
			class="an-socket an-sock-model"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'model',
						nodeId: data.id,
						element,
						payload: data.inputs.model!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">model</span>
	</div>
	<div class="an-in">
		<Ref
			class="an-socket an-sock-text"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'prompt',
						nodeId: data.id,
						element,
						payload: data.inputs.prompt!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">prompt ×</span>
	</div>
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
		<span class="an-port-label">ref ×</span>
	</div>

	<div class="an-body">
		<div class="an-row an-slider-row">
			<span class="an-label">Steps</span>
			<input
				type="range"
				min="1"
				max="100"
				value={data.steps}
				on:pointerdown|stopPropagation
				on:input={(e) => {
					data.steps = Number(e.currentTarget.value)
					touch()
				}}
			/>
			<span class="an-value">{data.steps}</span>
		</div>
		<div class="an-row an-slider-row">
			<span class="an-label">CFG</span>
			<input
				type="range"
				min="1"
				max="30"
				step="0.5"
				value={data.cfgScale}
				on:pointerdown|stopPropagation
				on:input={(e) => {
					data.cfgScale = Number(e.currentTarget.value)
					touch()
				}}
			/>
			<span class="an-value">{data.cfgScale}</span>
		</div>
		<div class="an-row">
			<span class="an-label">Seed</span>
			<input
				class="an-seed"
				type="text"
				inputmode="numeric"
				value={data.seed}
				on:pointerdown|stopPropagation
				on:input={(e) => {
					const v = parseInt(e.currentTarget.value, 10)
					if (!isNaN(v)) {
						data.seed = Math.max(0, v)
						touch()
					}
				}}
			/>
		</div>

		{#if data.busy}
			<div class="an-status">生成中…</div>
		{:else if data.error}
			<div class="an-error" title={data.error}>{data.error}</div>
		{/if}
		{#if data.resultUrl}
			<img class="an-preview" src={data.resultUrl} alt="生成结果" />
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
