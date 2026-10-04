<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import IconButton from '../../components/IconButton.svelte'
	import type { AreaExtra } from '../types'
	import { editNode, removeNodeCascade } from '../actions'
	import { noNodeDrag } from '../noNodeDrag'
	import { noCanvasWheel } from '../../noCanvasWheel'
	import type { PromptNode } from '../classes.svelte'

	let { data, emit }: { data: PromptNode; emit: (p: AreaExtra) => void } = $props()

	let count = $derived(data.text.length)
</script>

<NodeFrame
	nodeId={data.id}
	type="prompt"
	icon="prompt"
	name="Prompt"
	desc="提示词输入"
	selected={data.selected}
	ondelete={() => void removeNodeCascade(data.id)}
>
	{#snippet body()}
		<div class="prompt-wrap">
			<textarea
				class="ui-textarea"
				placeholder="描述你想生成的画面…"
				rows="4"
				value={data.text}
				use:noNodeDrag
				use:noCanvasWheel
				oninput={(e) =>
					editNode<PromptNode>(data.id, (n) => {
						n.text = (e.target as HTMLTextAreaElement).value
					})}
			></textarea>
			<div class="prompt-foot">
				<span class="count mono">{count}</span>
				{#if count > 0}
					<IconButton
						icon="x"
						label="清空"
						sm
						onclick={() =>
							editNode<PromptNode>(data.id, (n) => {
								n.text = ''
							})}
					/>
				{/if}
			</div>
		</div>
	{/snippet}

	{#snippet outputs()}
		<Port {data} {emit} side="output" port="text" label="text" tone="text" />
	{/snippet}
</NodeFrame>

<style>
	.prompt-wrap {
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.prompt-foot {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 6px;
		color: var(--ui-faint);
		font-size: 10px;
	}
</style>
