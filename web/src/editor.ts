import { NodeEditor } from 'rete'
import { AreaPlugin, AreaExtensions } from 'rete-area-plugin'
import { ConnectionPlugin, Presets as ConnectionPresets } from 'rete-connection-plugin'
import { SveltePlugin, Presets } from 'rete-svelte-plugin/5'
import ModelNodeComp from './nodes/components/ModelNode.svelte'
import PromptNodeComp from './nodes/components/PromptNode.svelte'
import LoadImageNodeComp from './nodes/components/LoadImageNode.svelte'
import GenerateNodeComp from './nodes/components/GenerateNode.svelte'
import PreviewNodeComp from './nodes/components/PreviewNode.svelte'
import ConnectionLine from './nodes/ConnectionLine.svelte'
import EmptySocket from './nodes/components/EmptySocket.svelte'
import { type Schemes, type AreaExtra } from './nodes/types'
import { connKeys } from './nodes/conn'
import { rt } from './runtime'
import { scheduleSave, scheduleViewSave } from './graphStore'

export function createEditor(container: HTMLElement) {
	const editor = new NodeEditor<Schemes>()
	const area = new AreaPlugin<Schemes, AreaExtra>(container)

	// 点阵网格画在随平移/缩放变换的 content 层上（容器背景不会动）；
	// 该层无尺寸，铺 100000² 的绝对定位层覆盖可视域，坐标原点保持 (0,0)
	area.area.content.holder.classList.add('ui-grid')

	const render = new SveltePlugin<Schemes, AreaExtra>()
	render.addPreset(
		Presets.classic.setup({
			customize: {
				node(context) {
					switch (context.payload.label) {
						case 'Model':
							return ModelNodeComp
						case 'Prompt':
							return PromptNodeComp
						case 'Image':
							return LoadImageNodeComp
						case 'Generate':
							return GenerateNodeComp
						case 'Preview':
							return PreviewNodeComp
					}
					return Presets.classic.Node
				},
				// 连线整只换掉：自绘曲线 + 按类型着色 + 流动动画
				connection: () => ConnectionLine,
				// 端口圆点由各节点组件自绘（.ui-socket，按类型着色）。这里必须给
				// 一个空组件而不是 null：renderer 只在有组件挂载时才发 rendered
				// 信号，socket 位置注册（连线端点定位）依赖该信号
				socket: () => EmptySocket,
			},
		}),
	)
	const connection = new ConnectionPlugin<Schemes, AreaExtra>()
	connection.addPreset(ConnectionPresets.classic.setup())

	editor.use(area)
	area.use(connection)
	area.use(render)

	// 右键：空白处打开添加菜单；节点上则选中该节点（配合 Delete/Backspace 删除）
	const selector = AreaExtensions.selector()
	const selectable = AreaExtensions.selectableNodes(area, selector, {
		accumulating: { active: () => false },
	})

	container.addEventListener('contextmenu', (e) => {
		e.preventDefault()
		const nodeId = (e.target as HTMLElement | null)
			?.closest<HTMLElement>('[data-node-id]')
			?.dataset.nodeId
		if (nodeId) {
			void selectable.select(nodeId, false)
			return
		}
		rt.onCanvasContextMenu?.(e.clientX, e.clientY)
	})

	// 删除选中节点：Delete / Backspace（输入框聚焦时忽略）。
	// removeNode 不会级联清理连线，须先删相邻连线。
	// 有选中连线时优先删连线。
	document.addEventListener('keydown', (e) => {
		if (e.key === 'Delete' || e.key === 'Backspace') {
			const tag = (document.activeElement as HTMLElement | null)?.tagName
			if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return
			if ((e.target as HTMLElement | null)?.closest?.('input, textarea, select, [contenteditable]')) return
			e.preventDefault()
			void (async () => {
				if (await removeSelectedConnection()) return
				const selected = editor.getNodes().filter((n) => n.selected)
				for (const node of selected) {
					for (const conn of editor.getConnections()) {
						const k = connKeys(conn as unknown as Record<string, unknown>)
						if (k.source === node.id || k.target === node.id) {
							await editor.removeConnection(conn.id)
						}
					}
					await editor.removeNode(node.id)
				}
			})()
			return
		}
		// Ctrl/Cmd + Enter：发起执行
		if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
			e.preventDefault()
			rt.onRunRequested?.()
		}
	})

	// 连线类型约束：两端 socket 实例相同才允许
	editor.addPipe((ctx) => {
		if (ctx.type === 'connectioncreate') {
			const c = ctx.data as unknown as Record<string, unknown>
			const { source, target, output, input } = connKeys(c)
			const src = editor.getNode(source)
			const dst = editor.getNode(target)
			const outIo = src?.outputs[output]
			const inIo = dst?.inputs[input]
			if (!outIo || !inIo || outIo.socket !== inIo.socket) return undefined
		}
		return ctx
	})

	// 持久化：结构变化立即排队，位置变化节流；连线变化同时通知参数区刷新
	const structural = new Set([
		'nodecreated',
		'noderemoved',
		'connectioncreated',
		'connectionremoved',
	])
	// 结构变化 → 保存结构文档；位置/视口变化 → 保存表现文档（两者节流）
	editor.addPipe((ctx) => {
		if (structural.has(ctx.type)) {
			scheduleSave()
			rt.onStructureChange?.()
		}
		return ctx
	})
	area.addPipe((ctx) => {
		if (ctx.type === 'nodetranslated' || ctx.type === 'translated' || ctx.type === 'zoomed') {
			scheduleViewSave()
		}
		return ctx
	})
	// 缩放百分比广播（视口控件订阅）+ 连线选中清理（点节点/空白处取消）
	area.addPipe((ctx) => {
		if (ctx.type === 'zoomed' || ctx.type === 'translated') notifyZoom(area.area.transform.k)
		if (ctx.type === 'nodepicked' || ctx.type === 'pointerdown') selectConnection(null)
		return ctx
	})

	rt.editor = editor
	rt.area = area
	return { editor, area }
}

/* ---------------- 连线选中（点击高亮 + Delete 删除） ---------------- */

const connSubs = new Set<(id: string | null) => void>()
let selectedConnection: string | null = null

export function subscribeConnection(cb: (id: string | null) => void): () => void {
	connSubs.add(cb)
	cb(selectedConnection)
	return () => connSubs.delete(cb)
}

export function selectConnection(id: string | null): void {
	if (id === selectedConnection) return
	selectedConnection = id
	for (const cb of connSubs) cb(id)
}

/** 删除当前选中的连线；无选中返回 false */
export async function removeSelectedConnection(): Promise<boolean> {
	if (!selectedConnection || !rt.editor) return false
	const id = selectedConnection
	selectConnection(null)
	await rt.editor.removeConnection(id)
	return true
}

/* ---------------- 视口控制（按钮 / 适配 / 缩放百分比） ---------------- */

const zoomSubs = new Set<(k: number) => void>()

export function subscribeZoom(cb: (k: number) => void): () => void {
	zoomSubs.add(cb)
	return () => zoomSubs.delete(cb)
}

function notifyZoom(k: number) {
	for (const cb of zoomSubs) cb(k)
}

/** 画布容器（#rete） */
function canvasEl(): HTMLElement | null {
	return (rt.area?.area.content.holder.parentElement as HTMLElement | null) ?? null
}

/** 以指定 client 点（缺省画布中心）为锚缩放 */
export function zoomBy(factor: number, clientX?: number, clientY?: number): void {
	const area = rt.area
	if (!area) return
	const rect = canvasEl()?.getBoundingClientRect()
	if (!rect) return
	const t = area.area.transform
	const px = clientX != null ? clientX - rect.left : rect.width / 2
	const py = clientY != null ? clientY - rect.top : rect.height / 2
	const next = Math.min(2.5, Math.max(0.2, t.k * factor))
	if (next === t.k) return
	const k = t.k
	t.k = next
	t.x = px - ((px - t.x) * next) / k
	t.y = py - ((py - t.y) * next) / k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${next})`
	notifyZoom(next)
	scheduleViewSave()
}

/** 缩放到 100% */
export function resetZoom(): void {
	const area = rt.area
	if (!area) return
	const rect = canvasEl()?.getBoundingClientRect()
	if (!rect) return
	const t = area.area.transform
	const px = rect.width / 2
	const py = rect.height / 2
	const k = t.k
	const next = 1
	t.k = next
	t.x = px - ((px - t.x) * next) / k
	t.y = py - ((py - t.y) * next) / k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${next})`
	notifyZoom(next)
	scheduleViewSave()
}

/** 全部节点收入视野 */
export function fitView(): void {
	const area = rt.area
	if (!area) return
	const views = [...area.nodeViews.values()]
	if (views.length === 0) return
	let minX = Infinity
	let minY = Infinity
	let maxX = -Infinity
	let maxY = -Infinity
	for (const v of views) {
		const w = v.element.offsetWidth || 220
		const h = v.element.offsetHeight || 140
		minX = Math.min(minX, v.position.x)
		minY = Math.min(minY, v.position.y)
		maxX = Math.max(maxX, v.position.x + w)
		maxY = Math.max(maxY, v.position.y + h)
	}
	const el = canvasEl()
	const w = el?.clientWidth || 1200
	const h = el?.clientHeight || 800
	const pad = 96
	const k = Math.min(1.4, Math.max(0.2, Math.min((w - pad) / (maxX - minX), (h - pad) / (maxY - minY))))
	const t = area.area.transform
	t.k = k
	t.x = (w - (maxX - minX) * k) / 2 - minX * k
	t.y = (h - (maxY - minY) * k) / 2 - minY * k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${k})`
	notifyZoom(k)
	scheduleViewSave()
}

export function currentZoom(): number {
	return rt.area?.area.transform.k ?? 1
}
