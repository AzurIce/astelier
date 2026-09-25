import { NodeEditor } from 'rete'
import { AreaPlugin, AreaExtensions } from 'rete-area-plugin'
import { ConnectionPlugin, Presets as ConnectionPresets } from 'rete-connection-plugin'
import { SveltePlugin, Presets } from 'rete-svelte-plugin/5'
import ModelNodeComp from './nodes/components/ModelNode.svelte'
import PromptNodeComp from './nodes/components/PromptNode.svelte'
import LoadImageNodeComp from './nodes/components/LoadImageNode.svelte'
import GenerateNodeComp from './nodes/components/GenerateNode.svelte'
import PreviewNodeComp from './nodes/components/PreviewNode.svelte'
import EmptySocket from './nodes/components/EmptySocket.svelte'
import { type Schemes, type AreaExtra } from './nodes/types'
import { connKeys } from './nodes/conn'
import { rt } from './runtime'
import { scheduleSave, scheduleViewSave } from './graphStore'

export function createEditor(container: HTMLElement) {
	const editor = new NodeEditor<Schemes>()
	const area = new AreaPlugin<Schemes, AreaExtra>(container)

	// 点阵网格画在随平移/缩放变换的 content 层上（容器背景不会动）；
	// 该层无尺寸，铺 100000² 的绝对定位层覆盖可视域
	area.area.content.holder.classList.add('an-grid')

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
				// 端口圆点由各节点组件自绘（.an-socket，按类型着色）。这里必须给
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
	// removeNode 不会级联清理连线，须先删相邻连线
	document.addEventListener('keydown', (e) => {
		if (e.key !== 'Delete' && e.key !== 'Backspace') return
		const tag = (document.activeElement as HTMLElement | null)?.tagName
		if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return
		const selected = editor.getNodes().filter((n) => n.selected)
		if (selected.length === 0) return
		e.preventDefault()
		void (async () => {
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
		if (structural.has(ctx.type)) scheduleSave()
		return ctx
	})
	area.addPipe((ctx) => {
		if (ctx.type === 'nodetranslated' || ctx.type === 'translated' || ctx.type === 'zoomed') {
			scheduleViewSave()
		}
		return ctx
	})

	rt.editor = editor
	rt.area = area
	return { editor, area }
}
