// 首次打开：种「Model + Prompt → Generate → Preview」最小闭环
import { GenerateNode, ModelNode, PreviewNode, PromptNode } from './nodes/classes'
import { connect } from './nodes/types'
import { rt } from './runtime'

export async function seedGraph() {
	const editor = rt.editor
	const area = rt.area
	if (!editor || !area) return

	const model = new ModelNode()
	const active = rt.providers.find((p) => p.id === rt.activeProvider) ?? rt.providers[0]
	model.provider = active?.id ?? ''
	model.modelId = active?.models[0] ?? ''

	const prompt = new PromptNode()
	prompt.text = 'a cat'

	const gen = new GenerateNode()
	const preview = new PreviewNode()

	await editor.addNode(model)
	await editor.addNode(prompt)
	await editor.addNode(gen)
	await editor.addNode(preview)

	// nodeViews 在渲染后才创建，translate 必须走 area 的异步接口
	await area.translate(model.id, { x: 60, y: 120 })
	await area.translate(prompt.id, { x: 60, y: 380 })
	await area.translate(gen.id, { x: 420, y: 200 })
	await area.translate(preview.id, { x: 780, y: 200 })

	await editor.addConnection(connect(model, 'model', gen, 'model'))
	await editor.addConnection(connect(prompt, 'text', gen, 'prompt'))
	await editor.addConnection(connect(gen, 'image', preview, 'image'))
}
