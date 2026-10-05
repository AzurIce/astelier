// 生成请求：直接返回当前调用的临时产物。
export interface GenerateParams {
	model: string
	prompt: string
	/** 统一键 → 标量（数字/字符串）；服务端按模型档案过滤与校验 */
	params?: Record<string, string | number | boolean | object | null>
	imageUrls?: string[]
}

export interface GenerateResult {
	/** 当前请求的临时产物；显式收藏之前不写入永久资产。 */
	imageUrls: string[]
}

export async function apiGenerate(params: GenerateParams): Promise<GenerateResult> {
	const res = await fetch('/api/generate', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify(params),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) {
		throw new Error(body?.error ?? `生成失败（${res.status}）`)
	}
	return body as GenerateResult
}

