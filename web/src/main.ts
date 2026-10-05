import { mount } from 'svelte'
import { applyDesign } from './ui/theme/state.svelte'
import './ui/theme/global.css'
import App from './app/App.svelte'
import { setWorkspaceStore, type WorkspaceStore } from './workspace/store'
import { createOpfsStore } from './workspace/opfs/store'
import { setGenerator, type ImageGenerator } from './generation/generator'
import { createDirectGenerator } from './generation/direct'

// 令牌在样式注入前落到 <html>，避免主题闪烁
applyDesign()

// 工作区装配。默认本地 OPFS + 浏览器直连；window.__atelierRuntime
// 是浏览器集成测试的注入缝（替身生图器、保存故障注入），正常加载为空。
const overrides = (window as {
	__atelierRuntime?: {
		wrapStore?: (store: WorkspaceStore) => WorkspaceStore
		generate?: (params: unknown) => Promise<unknown>
	}
}).__atelierRuntime

const store = overrides?.wrapStore ? overrides.wrapStore(createOpfsStore()) : createOpfsStore()
setWorkspaceStore(store)
setGenerator(
	overrides?.generate
		? { generate: overrides.generate as ImageGenerator['generate'] }
		: createDirectGenerator(),
)

const app = mount(App, { target: document.getElementById('app')! })

export default app
