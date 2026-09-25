<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { scheduleSave } from '../../graphStore'
	import { removeNodeCascade } from '../actions'
	import type { ModelNode } from '../classes'

	let { data, emit }: { data: ModelNode; emit: (p: AreaExtra) => void } = $props()

	let providers = $derived(rt.providers)
	let provider = $derived(providers.find((p) => p.id === data.provider) ?? providers[0])
	let models = $derived(provider?.models ?? [])

	function touch() {
		rt.area?.update('node', data.id)
		scheduleSave()
	}
</script>

<NodeFrame
	nodeId={data.id}
	type="model"
	icon="model"
	name="Model"
	desc={data.modelId || '未选择模型'}
	selected={data.selected}
	ondelete={() => removeNodeCascade(data.id)}
>
	{#snippet body()}
		<div class="field-row">
			<span class="field-label">Provider</span>
			<div class="field-value">
				<select
					class="ui-select"
					value={data.provider}
					onpointerdown={(e) => e.stopPropagation()}
					onchange={(e) => {
						data.provider = e.currentTarget.value
						// 换 provider 时 modelId 大概率失效，落到该 provider 第一个模型
						data.modelId =
							rt.providers.find((p) => p.id === data.provider)?.models[0] ?? ''
						touch()
					}}
				>
					{#each providers as p (p.id)}
						<option value={p.id}>{p.name}</option>
					{/each}
				</select>
			</div>
		</div>
		<div class="field-row">
			<span class="field-label">Model</span>
			<div class="field-value">
				<select
					class="ui-select"
					value={data.modelId}
					onpointerdown={(e) => e.stopPropagation()}
					onchange={(e) => {
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
	{/snippet}

	{#snippet outputs()}
		<Port {data} {emit} side="output" port="model" label="model" tone="model" />
	{/snippet}
</NodeFrame>
