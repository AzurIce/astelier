import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// 后端是本仓库的 axum 服务端（开发时默认 127.0.0.1:8230），
// /api 与 /asset 代理过去，其余走 Vite。
export default defineConfig({
	plugins: [svelte()],
	server: {
		proxy: {
			'/api': 'http://127.0.0.1:8230',
			'/asset': 'http://127.0.0.1:8230',
		},
	},
})
