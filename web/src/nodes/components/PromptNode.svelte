<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import type { PromptNode } from '../classes'

	export let data: PromptNode
	export let emit: (p: AreaExtra) => void
</script>

<div class="an-node" class:selected={data.selected}>
	<div class="an-title an-t-text">Prompt</div>
	<div class="an-body">
		<textarea
			placeholder="提示词…"
			rows="4"
			value={data.text}
			on:pointerdown|stopPropagation
			on:input={(e) => {
				data.text = e.currentTarget.value
				rt.area?.update('node', data.id)
			}}
		></textarea>
	</div>
	<div class="an-out">
		<span class="an-port-label">text</span>
		<Ref
			class="an-socket an-sock-text"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'output',
						key: 'text',
						nodeId: data.id,
						element,
						payload: data.outputs.text!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
	</div>
</div>
