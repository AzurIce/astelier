<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import type { AreaExtra } from '../types'
	import { backendRegistry } from '../../../backends/registry.svelte'
	import { editNode, removeNodeCascade } from '../actions'
	import { noNodeDrag } from '../../dom/noNodeDrag'
	import type { ModelNode } from '../model.svelte'
	let { data, emit }: { data: ModelNode; emit: (p: AreaExtra) => void } = $props()
	let source = $derived(backendRegistry.entries.find((entry) => entry.id === data.providerBackendId))
	let provider = $derived(source?.providers.find((provider) => provider.id === data.provider))
	let unavailable = $derived(!provider || source?.status !== 'online' || !!source?.providerError)
	let models = $derived(provider?.models ?? [])
	const valueOf = (backendId: string, providerId: string) => JSON.stringify([backendId, providerId])
</script>
<NodeFrame nodeId={data.id} type="model" icon="model" name="Model" desc={data.modelId || '未选择模型'} selected={data.selected} ondelete={() => void removeNodeCascade(data.id)}>
	{#snippet body()}
		<div class="field-row"><span class="field-label">Provider</span><div class="field-value">
			<select class="ui-select" aria-label="Provider" value={valueOf(data.providerBackendId, data.provider)} use:noNodeDrag onchange={(e) => {
				const [backendId, providerId] = JSON.parse(e.currentTarget.value)
				editNode<ModelNode>(data.id, (node) => { node.providerBackendId = backendId; node.provider = providerId; node.modelId = backendRegistry.entries.find((entry) => entry.id === backendId)?.providers.find((p) => p.id === providerId)?.models[0] ?? '' })
			}}>
				{#if !provider}<option value={valueOf(data.providerBackendId, data.provider)} disabled>{data.provider || '请选择 Provider'}（不可用）</option>{/if}
				{#each backendRegistry.entries as entry (entry.id)}
					<optgroup label={entry.kind === 'opfs' ? '本地 Provider' : entry.name} disabled={entry.status !== 'online' || !!entry.providerError}>
						{#each entry.providers as p (p.id)}<option value={valueOf(entry.id, p.id)}>{p.name}{entry.status !== 'online' ? '（离线）' : ''}</option>{/each}
					</optgroup>
				{/each}
			</select>
		</div></div>
		<div class="field-row"><span class="field-label">Model</span><div class="field-value">
			<select class="ui-select" aria-label="Model" value={data.modelId} disabled={unavailable} use:noNodeDrag onchange={(e) => editNode<ModelNode>(data.id, (node) => { node.modelId = e.currentTarget.value })}>
				{#if !models.includes(data.modelId)}<option value={data.modelId} disabled>{data.modelId || '未选择模型'}</option>{/if}
				{#each models as model (model)}<option value={model}>{model}</option>{/each}
			</select>
		</div></div>
		{#if unavailable}<div class="ui-error">所选 Provider 暂不可用</div>{/if}
	{/snippet}
	{#snippet outputs()}<Port {data} {emit} side="output" port="model" label="model" tone="model" />{/snippet}
</NodeFrame>
