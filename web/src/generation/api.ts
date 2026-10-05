// 生图请求类型与远端代理实现：POST {base}/api/generate，由服务端解析
// 密钥（env:VAR）、档案校验与参考图 URL（/store/ 等绝对地址）。
// 直接执行请用 generator.ts 注入的实例；本实现供远端工作区装配。
import type { ImageGenerator } from './generator'

export interface GenerateParams {
	model: string
	prompt: string
	/** 统一键 → 标量（数字/字符串）；实现按模型档案过滤与校验 */
	params?: Record<string, string | number | boolean | object | null>
	imageUrls?: string[]
}

export interface GenerateUsage {
	inputTokens?: number
	outputTokens?: number
	totalTokens?: number
	imageTokens?: number
}

export interface GenerateResult {
	/** 当前请求的临时产物；显式收藏之前不写入永久资产。 */
	imageUrls: string[]
	usage?: GenerateUsage
}

export function createHttpGenerator(baseUrl = ''): ImageGenerator {
	return {
		async generate(params: GenerateParams): Promise<GenerateResult> {
			const res = await fetch(`${baseUrl}/api/generate`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(params),
			})
			const body = await res.json().catch(() => null)
			if (!res.ok) {
				throw new Error(body?.error ?? `生成失败（${res.status}）`)
			}
			return body as GenerateResult
		},
	}
}
