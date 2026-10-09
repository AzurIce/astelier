import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vite'

// 前端是纯静态应用：本地工作区走 OPFS，远端工作区直接请求服务端绝对
// 地址，开发不再需要代理。远端服务需自行允许跨域（ASTELIER_CORS_ORIGINS）。
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
})
