import { NodeEditor } from 'rete'
import { AreaPlugin, AreaExtensions } from 'rete-area-plugin'
import { ConnectionPlugin, Presets as ConnectionPresets } from 'rete-connection-plugin'
import { ContextMenuPlugin, Presets as ContextMenuPresets } from 'rete-context-menu-plugin'
import { SveltePlugin, Presets } from 'rete-svelte-plugin/5'
import ModelNodeComp from './nodes/components/ModelNode.svelte'
import PromptNodeComp from './nodes/components/PromptNode.svelte'
import LoadImageNodeComp from './nodes/components/LoadImageNode.svelte'
import GenerateNodeComp from './nodes/components/GenerateNode.svelte'
import PreviewNodeComp from './nodes/components/PreviewNode.svelte'
import {
	GenerateNode,
	LoadImageNode,
	ModelNode,
	PreviewNode,
	PromptNode,
} from './nodes/classes'
import { type Schemes, type AreaExtra } from './nodes/types'
import { rt } from './runtime'
import { scheduleSave } from './persist'

/** 运行时 Connection 实例（source 为 Node 实例）→ 纯 id/键 */
export function connKeys(c: Record<string, unknown>) {
	const nodeId = (v: unknown) =>
		typeof v === 'string' ? v : String((v as { id?: string })?.id ?? '')
	return {
		source: nodeId(c.source),
		target: nodeId(c.target),
		output: String(c.sourceOutput ?? c.output ?? ''),
		input: String(c.targetInput ?? c.input ?? ''),
	}
}

export function createEditor(container: HTMLElement) {
	const editor = new NodeEditor<Schemes>()
	const area = new AreaPlugin<Schemes, AreaExtra>(container)

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
				// 端口圆点由各节点组件自绘（.an-socket，按类型着色）；
				// 不替换的话 classic 默认 Socket（24px 橄榄绿）会叠在自绘点上
				socket: () => null,
			},
		}),
	)

	const connection = new ConnectionPlugin<Schemes, AreaExtra>()
	connection.addPreset(ConnectionPresets.classic.setup())

	const contextMenu = new ContextMenuPlugin<Schemes>({
		items: ContextMenuPresets.classic.setup([
			['Model', () => new ModelNode()],
			['Prompt', () => new PromptNode()],
			['Image', () => new LoadImageNode()],
			['Generate', () => new GenerateNode()],
			['Preview', () => new PreviewNode()],
		]),
	})

	editor.use(area)
	area.use(connection)
	area.use(render)
	// context-menu 2.0.6 的 Signals 类型与 area Extra 不协变（运行时兼容，官方示例同用法）
	area.use(contextMenu as never)

	AreaExtensions.selectableNodes(area, AreaExtensions.selector(), {
		accumulating: { active: () => false },
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

	// 持久化：结构变化立即排队，位置变化节流
	const structural = new Set([
		'nodecreated',
		'noderemoved',
		'connectioncreated',
		'connectionremoved',
	])
	editor.addPipe((ctx) => {
		if (structural.has(ctx.type)) scheduleSave()
		return ctx
	})
	area.addPipe((ctx) => {
		if (ctx.type === 'nodetranslated') scheduleSave()
		return ctx
	})

	rt.editor = editor
	rt.area = area
	return { editor, area }
}
