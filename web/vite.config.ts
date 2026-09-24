import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// 后端是本仓库的 axum 服务端。dev 代理目标可用环境变量覆盖：
//   ATELIER_BACKEND=http://127.0.0.1:8232 bun run dev
// vite 自身端口（5173，被占时自动 5174/…）与后端无关。
const backend = process.env.ATELIER_BACKEND ?? 'http://127.0.0.1:8230'

export default defineConfig({
	plugins: [svelte()],
	server: {
		proxy: {
			'/api': backend,
			'/asset': backend,
		},
	},
})
