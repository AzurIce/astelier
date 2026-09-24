import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vitejs.dev/config/
// 后端是本仓库的 axum 服务端（开发时默认 127.0.0.1:8080），
// /api 与 /asset 代理过去，其余走 Vite。
export default defineConfig({
	plugins: [react()],
	server: {
		proxy: {
			'/api': 'http://127.0.0.1:8230',
			'/asset': 'http://127.0.0.1:8230',
		},
	},
})
