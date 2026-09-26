<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { editNode, removeNodeCascade } from '../actions'
	import type { ModelNode } from '../classes'

	let { data, emit }: { data: ModelNode; emit: (p: AreaExtra) => void } = $props()

	let providers = $derived(rt.providers)
	let provider = $derived(providers.find((p) => p.id === data.provider) ?? providers[0])
	let models = $derived(provider?.models ?? [])
</script>

<NodeFrame
	nodeId={data.id}
	type="model"
	icon="model"
	name="Model"
	desc={data.modelId || '未选择模型'}
	selected={data.selected}
	ondelete={() => void removeNodeCascade(data.id)}
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
						const providerId = (e.target as HTMLSelectElement).value
						editNode<ModelNode>(data.id, (n) => {
							n.provider = providerId
							// 换 provider 时 modelId 大概率失效，落到该 provider 第一个模型
							n.modelId = rt.providers.find((p) => p.id === providerId)?.models[0] ?? ''
						})
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
					onchange={(e) =>
						editNode<ModelNode>(data.id, (n) => {
							n.modelId = (e.target as HTMLSelectElement).value
						})}
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
