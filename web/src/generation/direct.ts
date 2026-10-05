// 浏览器直连生图（移植自 Rust adapter.rs + api.rs generate 流程）：
// 从本地 Provider 配置读取 Base URL 与密钥，文生图发 JSON 到
// {base}/images/generations，参考图编辑发 multipart 到 {base}/images/edits。
// 请求与结果只存活于本次调用，不写任何档案；CORS spike 已验证入口可用。
import { workspaceStore } from '../workspace/store'
import type { GenerateResult, ImageGenerator } from './generator'
import type { GenerateParams } from './api'
import { coerceParamValue, mergedProfile, validateRequest, withDefaults, type ParamMap } from './profiles'
import {
	base64ToBytes,
	bytesToDataUrl,
	collectResultItems,
	editsFields,
	extractErrorMessage,
	generationsBody,
	imageFileName,
	imageMime,
	imagePartName,
	nonJsonError,
	parseDataImageUrl,
	parseUsage,
	type InputImage,
} from './protocol'
import { sniffExt } from '../images/sniff'

const REQUEST_TIMEOUT = 600_000
const DOWNLOAD_TIMEOUT = 120_000

export function createDirectGenerator(): ImageGenerator {
	return {
		async generate(params: GenerateParams): Promise<GenerateResult> {
			const config = await workspaceStore().loadConfig()
			// model 解析：`provider:model` 前缀命中已配置 provider 才拆开，
			// 否则整体视作 model、走当前激活 provider
			let provider = config.providers.find((p) => p.id === config.active_provider)
			let modelId = params.model
			const colon = params.model.indexOf(':')
			if (colon > 0) {
				const hit = config.providers.find((p) => p.id === params.model.slice(0, colon))
				if (hit) {
					provider = hit
					modelId = params.model.slice(colon + 1)
				}
			}
			if (!provider) throw new Error('没有可用的 Provider，请先在设置里配置')

			const profile = mergedProfile(modelId, provider.overrides?.[modelId])
			if (!params.prompt.trim()) throw new Error('Prompt 为空')

			const paramMap: ParamMap = {}
			for (const [key, value] of Object.entries(params.params ?? {})) {
				const coerced = coerceParamValue(value)
				if (coerced) paramMap[key] = coerced
			}
			const normalized = withDefaults(profile, paramMap)

			const images: InputImage[] = []
			for (const url of params.imageUrls ?? []) {
				if (!url) continue
				if (url.startsWith('data:')) {
					const parsed = parseDataImageUrl(url)
					if (!parsed) throw new Error('参考图需要 PNG/JPEG/WebP/GIF 的 base64 data URL')
					images.push(parsed)
				} else if (url.startsWith('blob:')) {
					const res = await fetch(url)
					if (!res.ok) throw new Error(`读取参考图失败（HTTP ${res.status}）`)
					const bytes = new Uint8Array(await res.arrayBuffer())
					images.push({ bytes, ext: sniffExt(bytes) })
				} else {
					throw new Error('仅支持会话内图片（data: / blob:）；库与图内图片会先解析为会话引用')
				}
			}

			const invalid = validateRequest(profile, modelId, normalized, images.length)
			if (invalid) throw new Error(invalid)

			const base = provider.base_url.trim().replace(/\/+$/, '')
			if (!base) throw new Error('Provider 未配置 Base URL')
			const apiKey = provider.api_key.trim()
			if (!apiKey) throw new Error('Provider 未配置 API Key（在设置里填写）')

			const signal = AbortSignal.timeout(REQUEST_TIMEOUT)
			let response: Response
			if (!images.length) {
				response = await fetch(`${base}/images/generations`, {
					method: 'POST',
					headers: { Authorization: `Bearer ${apiKey}`, 'Content-Type': 'application/json' },
					body: JSON.stringify(generationsBody(modelId, params.prompt, normalized, profile)),
					signal,
				})
			} else {
				const form = new FormData()
				for (const [key, value] of editsFields(modelId, params.prompt, normalized, profile)) form.append(key, value)
				const part = imagePartName(images.length)
				images.forEach((image, i) => {
					form.append(part, new File([image.bytes], imageFileName(i, image.ext), { type: imageMime(image.ext) }))
				})
				response = await fetch(`${base}/images/edits`, {
					method: 'POST',
					headers: { Authorization: `Bearer ${apiKey}` },
					body: form,
					signal,
				})
			}
			return await parseGenerationResponse(response)
		},
	}
}

async function parseGenerationResponse(res: Response): Promise<GenerateResult> {
	const text = await res.text()
	let json: unknown
	try {
		json = JSON.parse(text)
	} catch {
		throw new Error(nonJsonError(res.status, text))
	}
	if (!res.ok) throw new Error(extractErrorMessage(json, res.status))

	const items = collectResultItems(json)
	if (!items.length) throw new Error('响应中没有图片数据')
	const imageUrls: string[] = []
	for (const item of items) {
		if ('b64' in item) imageUrls.push(bytesToDataUrl(base64ToBytes(item.b64)))
		else {
			const download = await fetch(item.url, { signal: AbortSignal.timeout(DOWNLOAD_TIMEOUT) })
			if (!download.ok) {
				throw new Error(`拉取结果图失败（HTTP ${download.status}；若图片服务器不允许跨域读取，可改用返回 b64 的模型）`)
			}
			imageUrls.push(bytesToDataUrl(new Uint8Array(await download.arrayBuffer())))
		}
	}
	const usage = parseUsage(json)
	return {
		imageUrls,
		...(usage ? { usage: { inputTokens: usage.input_tokens, outputTokens: usage.output_tokens, totalTokens: usage.total_tokens, imageTokens: usage.image_tokens } } : {}),
	}
}
