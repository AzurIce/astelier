import { mount } from 'svelte'
import { applyDesign } from './ui/theme/state.svelte'
import './ui/theme/global.css'
import App from './app/App.svelte'
import { initializeBackends } from './backends/registry.svelte'

applyDesign()
initializeBackends()

const app = mount(App, { target: document.getElementById('app')! })

export default app
