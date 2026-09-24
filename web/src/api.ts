// 后端 REST API 封装（错误统一抛 Error(message)）

export interface ProviderInfo {
	id: string
	name: string
	models: string[]
}

export async function fetchConfig(): Promise<{
	providers: ProviderInfo[]
	active_provider: string
}> {
	const res = await fetch('/api/config')
	if (!res.ok) throw new Error(`读取配置失败（${res.status}）`)
	return res.json()
}

export interface GenerateParams {
	model: string
	prompt: string
	negativePrompt?: string
	steps?: number
	cfgScale?: number
	seed?: number
	referenceImageUrl?: string
}

export interface GenerateResult {
	imageUrl: string
	seed: number
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

export async function uploadAsset(file: File): Promise<string> {
	const res = await fetch(`/api/assets?filename=${encodeURIComponent(file.name)}`, {
		method: 'POST',
		body: file,
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return `/asset/${body.id}.${body.ext}`
}
