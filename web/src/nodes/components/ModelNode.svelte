<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { scheduleSave } from '../../persist'
	import type { ModelNode } from '../classes'

	export let data: ModelNode
	export let emit: (p: AreaExtra) => void

	$: provider = rt.providers.find((p) => p.id === data.provider) ?? rt.providers[0]
	$: models = provider?.models ?? []

	function touch() {
		rt.area?.update('node', data.id)
		scheduleSave()
	}
</script>

<div class="an-node" class:selected={data.selected} data-node-id={data.id}>
	<div class="an-title an-t-model">Model</div>
	<div class="an-body">
		<div class="an-row">
			<span class="an-label">Provider</span>
			<select
				value={data.provider}
				on:pointerdown|stopPropagation
				on:change={(e) => {
					data.provider = e.currentTarget.value
					// 换 provider 时 modelId 大概率失效，落到该 provider 第一个模型
					data.modelId = rt.providers.find((p) => p.id === data.provider)?.models[0] ?? ''
					touch()
				}}
			>
				{#each rt.providers as p (p.id)}
					<option value={p.id}>{p.name}</option>
				{/each}
			</select>
		</div>
		<div class="an-row">
			<span class="an-label">Model</span>
			<select
				value={data.modelId}
				on:pointerdown|stopPropagation
				on:change={(e) => {
					data.modelId = e.currentTarget.value
					touch()
				}}
			>
				{#each models as m (m)}
					<option value={m}>{m}</option>
				{/each}
			</select>
		</div>
	</div>
	<div class="an-out">
		<span class="an-port-label">model</span>
		<Ref
			class="an-socket an-sock-model"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'output',
						key: 'model',
						nodeId: data.id,
						element,
						payload: data.outputs.model!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
	</div>
</div>
