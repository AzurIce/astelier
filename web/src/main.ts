import { mount } from 'svelte'
import { applyDesign } from './design/store.svelte'
import './index.css'
import App from './App.svelte'

// 令牌在样式注入前落到 <html>，避免主题闪烁
applyDesign()

const app = mount(App, { target: document.getElementById('app')! })

export default app
