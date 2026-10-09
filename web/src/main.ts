import { mount } from 'svelte'
import { applyDesign } from './ui/theme/state.svelte'
import './ui/theme/global.css'
import App from './app/App.svelte'
import { initializeBackends } from './backends/registry.svelte'

applyDesign()

try {
	initializeBackends()
} catch (error) {
	console.error('后端连接装配失败', error)
}

const app = mount(App, { target: document.getElementById('app')! })

export default app
