<script lang="ts">
	import { onMount } from 'svelte'
	import { rt, runningNodes } from '../runtime'
	import { selectConnection, subscribeConnection } from '../editor'

	// 自绘连线（rete classic preset 的 connection 渲染替换件）。
	// 注意：ConnectionWrapper 把连接的字段**平铺**成 props（{...data}），
	// 所以这里直接收 id / source / sourceOutput / isPseudo。
	// 单击选中（高亮），Delete/Backspace 删除选中的连线；悬停仅提亮，不加按钮。
	let {
		id,
		source,
		sourceOutput,
		isPseudo,
		start,
		end,
	}: {
		id: string
		source: string
		sourceOutput: string
		isPseudo?: boolean
		start: { x: number; y: number }
		end: { x: number; y: number }
	} = $props()

	let selected = $state(false)
	let hovered = $state(false)

	onMount(() => subscribeConnection((selId) => (selected = selId === id)))

	const sourceNode = $derived(rt.editor?.getNode(source))
	const socket = $derived(
		sourceNode?.outputs[sourceOutput]?.socket as { name?: string } | undefined,
	)
	const type = $derived(socket?.name ?? null)
	const flowing = $derived(!isPseudo && runningNodes.has(source))

	// 三次贝塞尔：端口固定在左右缘，统一用水平 S 曲线（n8n/Blender 做法）。
	// 控制点只做水平偏移，垂直方向自然过渡，不甩环。
	const curve = $derived.by(() => {
		const sx = start.x
		const sy = start.y
		const ex = end.x
		const ey = end.y
		const k = Math.max(60, Math.abs(ex - sx) * 0.6)
		return {
			d: `M ${sx} ${sy} C ${sx + k} ${sy}, ${ex - k} ${ey}, ${ex} ${ey}`,
			p1: { x: sx + k, y: sy },
			p2: { x: ex - k, y: ey },
		}
	})
	const d = $derived(curve.d)
</script>

<svg
	class="ui-conn {type ? `t-${type}` : ''}"
	class:pseudo={isPseudo}
	class:selected
	class:flowing
	class:hovered
>
	<!-- 视觉线 -->
	<path class="wire" {d} />
	<!-- 命中区（悬停提亮 + 单击选中） -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<path
		class="wire hit"
		{d}
		onpointerenter={() => (hovered = true)}
		onpointerleave={() => (hovered = false)}
		onclick={() => selectConnection(id)}
	/>
</svg>

<style>
	.ui-conn {
		position: absolute;
		left: 0;
		top: 0;
		width: 9999px;
		height: 9999px;
		overflow: visible;
		pointer-events: none;
		/* 负层：走在所有节点下方。连线进入节点的部分被节点主体遮住，
		   到边框即止，不会压过端口标签；网格背景仍在线之下 */
		z-index: -1;
	}
	.ui-conn path.wire {
		fill: none;
		stroke-width: 2;
		stroke-linecap: round;
		vector-effect: non-scaling-stroke;
		transition: stroke var(--ui-mid), stroke-width var(--ui-mid);
	}
	.ui-conn path.wire.hit {
		stroke: transparent;
		stroke-width: 14;
		pointer-events: stroke;
		cursor: pointer;
	}
	/* 悬停提亮 / 选中高亮 */
	.ui-conn.hovered path.wire:not(.hit) {
		stroke-width: 2.8;
		filter: brightness(1.3);
	}
	.ui-conn.selected path.wire:not(.hit) {
		stroke: var(--ui-accent) !important;
		stroke-width: 3;
	}
	/* 按端口类型着色 */
	.ui-conn.t-model path.wire:not(.hit) {
		stroke: var(--ui-sock-model);
	}
	.ui-conn.t-text path.wire:not(.hit) {
		stroke: var(--ui-sock-text);
	}
	.ui-conn.t-image path.wire:not(.hit) {
		stroke: var(--ui-sock-image);
	}
	/* 默认色 */
	.ui-conn:not(.t-model):not(.t-text):not(.t-image) path.wire:not(.hit) {
		stroke: var(--ui-conn);
	}

	/* 拖拽中的伪连线 */
	.ui-conn.pseudo path.wire:not(.hit) {
		stroke-dasharray: 7 5;
		animation: dash-flow 0.6s linear infinite;
	}
	@keyframes dash-flow {
		to {
			stroke-dashoffset: -12;
		}
	}
	/* 生成进行中的流动线 */
	.ui-conn.flowing path.wire:not(.hit) {
		stroke-dasharray: 10 6;
		animation: dash-flow 1s linear infinite;
		filter: drop-shadow(0 0 4px var(--ui-accent-fade));
	}
</style>
