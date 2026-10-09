// ImageGenerator：生图的抽象边界。接收模型、prompt、参数与参考图 URL，
// 返回本次调用的临时图片。实现有浏览器直连（direct.ts）与远端执行（remote.ts）；
// 每个 Provider 拥有自己的执行实例，测试通过 window.__astelierRuntime 注入替身。
export interface GenerateParams {
	model: string
	prompt: string
	params?: Record<string, string | number | boolean | object | null>
	imageUrls?: string[]
}

export interface GenerateResult {
	imageUrls: string[]
	usage?: { inputTokens?: number; outputTokens?: number; totalTokens?: number; imageTokens?: number }
}

export interface ImageGenerator {
	generate(params: GenerateParams): Promise<GenerateResult>
}
