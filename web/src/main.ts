import { mount } from 'svelte'
import { applyDesign } from './ui/theme/state.svelte'
import './ui/theme/global.css'
import App from './app/App.svelte'
import { setWorkspaceStore, type WorkspaceStore } from './workspace/store'
import { createOpfsStore } from './workspace/opfs/store'
import { createHttpStore } from './workspace/httpStore'
import { currentWorkspaceChoice } from './workspace/selection.svelte'
import { setGenerator, type ImageGenerator } from './generation/generator'
import { createDirectGenerator } from './generation/direct'
import { createHttpGenerator } from './generation/api'

// 令牌在样式注入前落到 <html>，避免主题闪烁
applyDesign()

// 工作区装配：本地 OPFS + 浏览器直连，或远端服务 + 服务端代理生图。
// window.__atelierRuntime 是浏览器集成测试的注入缝（替身生图器、保存
// 故障注入），正常加载为空。
const choice = currentWorkspaceChoice()
const overrides = (window as {
	__atelierRuntime?: {
		wrapStore?: (store: WorkspaceStore) => WorkspaceStore
		generate?: (params: unknown) => Promise<unknown>
	}
}).__atelierRuntime

const store = choice.kind === 'http' ? createHttpStore(choice.baseUrl) : createOpfsStore()
setWorkspaceStore(overrides?.wrapStore ? overrides.wrapStore(store) : store)
setGenerator(
	overrides?.generate
		? { generate: overrides.generate as ImageGenerator['generate'] }
		: choice.kind === 'http'
			? createHttpGenerator(choice.baseUrl)
			: createDirectGenerator(),
)

const app = mount(App, { target: document.getElementById('app')! })

export default app
