// ImageGenerator：生图的抽象边界。接收模型、prompt、参数与参考图 URL，
// 返回本次调用的临时图片。实现有浏览器直连（direct.ts）与远端代理（api.ts）；
// 实例由 main.ts 按工作区装配，测试通过 window.__atelierRuntime 注入替身。
import type { GenerateParams, GenerateResult } from './api'

export type { GenerateParams, GenerateResult }

export interface ImageGenerator {
	generate(params: GenerateParams): Promise<GenerateResult>
}

let current: ImageGenerator | null = null

export function setGenerator(generator: ImageGenerator): void {
	current = generator
}

export function imageGenerator(): ImageGenerator {
	if (!current) throw new Error('生图器未初始化（应由 main.ts 装配）')
	return current
}
