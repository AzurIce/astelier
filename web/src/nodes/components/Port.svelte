<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import type { ClassicPreset } from 'rete'
	import type { NodeTypes } from '../classes'

	// 端口行：Ref 元素 + 类型色点 + 标签。输入居左、输出居右。
	// 必须继续经 emit({type:'render'}) 注册元素位置（连线端点定位依赖）。
	let {
		data,
		emit,
		side,
		port,
		label,
		tone,
	}: {
		data: NodeTypes
		emit: (p: AreaExtra) => void
		side: 'input' | 'output'
		port: string
		label: string
		tone: 'model' | 'text' | 'image'
	} = $props()

	const io = $derived(
		side === 'input'
			? (data.inputs as Record<string, { socket: ClassicPreset.Socket }>)[port]
			: (data.outputs as Record<string, { socket: ClassicPreset.Socket }>)[port],
	)
</script>

<div class="port-row {side}">
	{#if side === 'input'}
		<Ref
			class="ui-socket {tone}"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: port,
						nodeId: data.id,
						element,
						payload: io.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="port-label"><i style:background={`var(--ui-sock-${tone})`}></i>{label}</span>
	{:else}
		<span class="port-label">{label}<i style:background={`var(--ui-sock-${tone})`}></i></span>
		<Ref
			class="ui-socket {tone}"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'output',
						key: port,
						nodeId: data.id,
						element,
						payload: io.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
	{/if}
</div>
