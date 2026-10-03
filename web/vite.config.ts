import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// 后端是本仓库的 axum 服务端。dev 代理目标可用环境变量覆盖：
//   ATELIER_BACKEND=http://127.0.0.1:8232 bun run dev
// vite 自身端口（5173，被占时自动 5174/…）与后端无关。
const backend = process.env.ATELIER_BACKEND ?? 'http://127.0.0.1:8230'

export default defineConfig({
	plugins: [svelte()],
	css: {
		preprocessorOptions: {
			scss: {
				// 上游 rete-svelte-plugin 的 preset 组件（context-menu/classic 等）
				// 仍用 @import 与 legacy color 函数，Dart Sass 3.0 前刷弃用警告，
				// 静默之，保持 dev 输出干净（上游修复前不影响构建）
				silenceDeprecations: ['import', 'global-builtin', 'color-functions'],
			},
		},
	},
	server: {
		proxy: {
			'/api': backend,
			'/asset': backend,
			// image store 静态寻址（dev 下同样代理到后端，否则落 SPA 返回 HTML 导致裂图）
			'/gstore': backend,
			// 全局库静态寻址（同 gstore，dev 下必须代理）
			'/store': backend,
		},
	},
})
