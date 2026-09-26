<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import type { ClassicPreset } from 'rete'
	import type { NodeTypes } from '../classes'

	// 端口行：socket 圆点骑在节点左右边框上，标签贴在圆点内侧。
	// 输入（左列）：圆点在左边缘；输出（右列）：圆点在右边缘。
	// init/unmount 保持模板内联箭头——rete-svelte-plugin 的重渲染链路
	// 依赖 Port 每次重渲染重新触发 socket 的 render 信号（改成稳定闭包
	// 会让父组件的外部 prop 更新不再生效，选中态僵死）。
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
		<span class="port-label">{label}</span>
	{:else}
		<span class="port-label">{label}</span>
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
