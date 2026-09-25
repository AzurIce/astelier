<script lang="ts">
	import { rt, runningNodes } from '../runtime'
	import { connKeys } from './conn'
	import type { Schemes } from './types'

	// 自绘连线（rete classic preset 的 connection 渲染替换件）。
	// 着色 = 源端口 socket 类型；悬停出现删除钮；Generate 运行中的出线流动。
	let { data, start, end }: { data: Schemes['Connection'] & { isPseudo?: boolean }; start: { x: number; y: number }; end: { x: number; y: number } } = $props()

	let hovered = $state(false)

	// 源节点 id 与端口键 → socket 类型。
	// 注意：拖拽中的伪连线（data 只有 isPseudo，没有 source/target）也要渲染。
	const conn = $derived((data ?? {}) as unknown as Record<string, unknown>)
	const isPseudo = $derived(Boolean(conn.isPseudo))
	const k = $derived(connKeys(conn))
	const srcId = $derived(k.source || null)
	const sourceNode = $derived(srcId ? rt.editor?.getNode(srcId) : undefined)
	const socket = $derived(sourceNode?.outputs[k.output]?.socket as { name?: string } | undefined)
	const type = $derived(socket?.name ?? null)
	const flowing = $derived(Boolean(srcId && runningNodes.has(srcId)))

	// 三次贝塞尔：按连接方向自适应。
	// 端口位于节点上/下边缘（行式布局）时走竖向 S；左右分布时走横向 S，
	// 避免传统水平贝塞尔在上下连接时甩出大回环。
	const curve = $derived.by(() => {
		const sx = start.x
		const sy = start.y
		const ex = end.x
		const ey = end.y
		if (Math.abs(ex - sx) >= Math.abs(ey - sy)) {
			const k = Math.max(48, Math.abs(ex - sx) * 0.55)
			return {
				d: `M ${sx} ${sy} C ${sx + k} ${sy}, ${ex - k} ${ey}, ${ex} ${ey}`,
				p1: { x: sx + k, y: sy },
				p2: { x: ex - k, y: ey },
			}
		}
		const k = Math.max(48, Math.abs(ey - sy) * 0.55)
		return {
			d: `M ${sx} ${sy} C ${sx} ${sy + k}, ${ex} ${ey - k}, ${ex} ${ey}`,
			p1: { x: sx, y: sy + k },
			p2: { x: ex, y: ey - k },
		}
	})
	const d = $derived(curve.d)
	// 删除钮落点：三次曲线参数中点 B(0.5)
	const mid = $derived.by(() => ({
		x: (start.x + 3 * curve.p1.x + 3 * curve.p2.x + end.x) / 8,
		y: (start.y + 3 * curve.p1.y + 3 * curve.p2.y + end.y) / 8,
	}))

	async function remove() {
		const editor = rt.editor
		if (!editor) return
		await editor.removeConnection(data.id)
	}
</script>

<svg class="ui-conn {type ? `t-${type}` : ''}" class:pseudo={isPseudo} class:flowing={!isPseudo && flowing}>
	<!-- 视觉线 -->
	<path class="wire" {d} />
	<!-- 命中区（悬停显示删除钮） -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<path
		class="wire hit"
		{d}
		onpointerenter={() => (hovered = true)}
		onpointerleave={() => (hovered = false)}
	/>
	{#if hovered && !isPseudo}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<g class="del" role="button" tabindex="-1" aria-label="删除连线" onclick={remove}>
			<circle cx={mid.x} cy={mid.y} r="8" />
			<path d="M {mid.x - 3} {mid.y - 3} L {mid.x + 3} {mid.y + 3} M {mid.x + 3} {mid.y - 3} L {mid.x - 3} {mid.y + 3}" />
		</g>
	{/if}
	<!-- 端点小圆：盖住 socket 边缘，视觉更实 -->
	<circle class="port" cx={start.x} cy={start.y} r="2.6" />
	<circle class="port" cx={end.x} cy={end.y} r="2.6" />
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
		z-index: 0;
	}
	/* 连线插在节点之下；命中区单独允许事件 */
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
	.ui-conn circle.port {
		fill: var(--ui-conn);
		vector-effect: non-scaling-stroke;
		transition: fill var(--ui-mid);
	}
	/* 按端口类型着色 */
	.ui-conn.t-model path.wire:not(.hit),
	.ui-conn.t-model circle.port {
		stroke: var(--ui-sock-model);
	}
	.ui-conn.t-model circle.port {
		fill: var(--ui-sock-model);
	}
	.ui-conn.t-text path.wire:not(.hit),
	.ui-conn.t-text circle.port {
		stroke: var(--ui-sock-text);
	}
	.ui-conn.t-text circle.port {
		fill: var(--ui-sock-text);
	}
	.ui-conn.t-image path.wire:not(.hit),
	.ui-conn.t-image circle.port {
		stroke: var(--ui-sock-image);
	}
	.ui-conn.t-image circle.port {
		fill: var(--ui-sock-image);
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

	/* 悬停删除钮 */
	.ui-conn .del {
		cursor: pointer;
		pointer-events: auto;
	}
	.ui-conn .del circle {
		fill: var(--ui-panel);
		stroke: var(--ui-danger);
		stroke-width: 1.5;
		vector-effect: non-scaling-stroke;
		cursor: pointer;
	}
	.ui-conn .del path {
		stroke: var(--ui-danger);
		stroke-width: 1.6;
		stroke-linecap: round;
		vector-effect: non-scaling-stroke;
		pointer-events: none;
	}
	.ui-conn .del:hover circle {
		fill: var(--ui-danger);
	}
	.ui-conn .del:hover path {
		stroke: #fff;
	}
</style>
