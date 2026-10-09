import type { GenerateParams, GenerateResult, ImageGenerator } from './generator'
import { bytesToDataUrl } from './protocol'

/** Every reference is transferred as bytes; the server never receives a browser blob URL. */
export async function transferableImages(urls: string[]): Promise<string[]> {
	return Promise.all(urls.map(async (url) => {
		if (url.startsWith('data:')) return url
		const res = await fetch(url)
		if (!res.ok) throw new Error(`读取参考图失败（HTTP ${res.status}）`)
		return bytesToDataUrl(new Uint8Array(await res.arrayBuffer()))
	}))
}

export function createRemoteGenerator(baseUrl: string, providerId: string): ImageGenerator {
	return {
		async generate(params: GenerateParams): Promise<GenerateResult> {
			const imageUrls = await transferableImages(params.imageUrls ?? [])
			const res = await fetch(`${baseUrl}/api/providers/${encodeURIComponent(providerId)}/generate`, {
				method: 'POST', headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ ...params, imageUrls }), signal: AbortSignal.timeout(600_000),
			})
			const body = await res.json().catch(() => null)
			if (!res.ok) throw new Error(body?.error ?? `生成失败（HTTP ${res.status}）`)
			if (!Array.isArray(body?.imageUrls) || !body.imageUrls.every((url: unknown) => typeof url === 'string')) throw new Error('生成响应格式不正确')
			return body
		},
	}
}
