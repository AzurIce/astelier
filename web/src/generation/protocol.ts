// OpenAI Images 兼容协议的纯逻辑层（移植自 Rust adapter.rs）：
// 请求体组装（generations JSON / edits multipart 字段）、响应解析
// （b64_json / url / usage / 错误提取）与图片字节 ↔ data URL 转换。
// 网络发送在 direct.ts，远端代理在 http.ts。

import { imageMime, sniffExt } from '../images/sniff'
import { paramsToBody, type ModelProfile, type ParamMap } from './profiles'

/** 本次调用的参考图字节；不复制到任何永久存储 */
export interface InputImage {
	bytes: Uint8Array<ArrayBuffer>
	ext: string
}

export interface Usage {
	input_tokens?: number
	output_tokens?: number
	total_tokens?: number
	image_tokens?: number
}

/** 文生图请求体：JSON 直发 {base}/images/generations */
export function generationsBody(modelId: string, prompt: string, params: ParamMap, profile: ModelProfile): Record<string, string | number> {
	return { model: modelId, prompt, ...paramsToBody(profile, params) }
}

/** 编辑请求的文本字段（multipart）：model、prompt 与各参数（一律转字符串） */
export function editsFields(modelId: string, prompt: string, params: ParamMap, profile: ModelProfile): Array<[string, string]> {
	const fields: Array<[string, string]> = [
		['model', modelId],
		['prompt', prompt],
	]
	for (const [key, value] of Object.entries(paramsToBody(profile, params))) {
		fields.push([key, typeof value === 'string' ? value : JSON.stringify(value)])
	}
	return fields
}

/** multipart 图片字段名：多图 image[]，单图 image */
export function imagePartName(count: number): string {
	return count > 1 ? 'image[]' : 'image'
}

export function imageFileName(index: number, ext: string): string {
	return `image-${index}.${ext}`
}

export { imageMime }

/** 解析上游 usage（snake_case，与 OpenAI 一致） */
export function parseUsage(json: unknown): Usage | undefined {
	if (typeof json !== 'object' || json === null) return undefined
	const usage = (json as Record<string, unknown>).usage
	if (typeof usage !== 'object' || usage === null) return undefined
	const u = usage as Record<string, unknown>
	const num = (v: unknown): number | undefined => (typeof v === 'number' && Number.isFinite(v) ? v : undefined)
	const details = typeof u.input_tokens_details === 'object' && u.input_tokens_details !== null ? (u.input_tokens_details as Record<string, unknown>) : {}
	const out: Usage = {
		input_tokens: num(u.input_tokens),
		output_tokens: num(u.output_tokens),
		total_tokens: num(u.total_tokens),
		image_tokens: num(details.image_tokens),
	}
	return Object.values(out).some((v) => v !== undefined) ? out : undefined
}

/** 上游错误文案：优先 /error/message，否则 HTTP 状态码 */
export function extractErrorMessage(json: unknown, status: number): string {
	if (typeof json === 'object' && json !== null) {
		const error = (json as Record<string, unknown>).error
		if (typeof error === 'object' && error !== null) {
			const message = (error as Record<string, unknown>).message
			if (typeof message === 'string' && message) return message
		}
	}
	return `HTTP ${status}`
}

/** 响应不是 JSON 时的错误（截断前 300 字符，与 Rust 侧一致） */
export function nonJsonError(status: number, text: string): string {
	return `HTTP ${status} · 响应不是 JSON：${Array.from(text).slice(0, 300).join('')}`
}

/** 结果条目：b64 优先，其次 url（url 需要二次跨域下载） */
export type ResultItem = { b64: string } | { url: string }

export function collectResultItems(json: unknown): ResultItem[] {
	if (typeof json !== 'object' || json === null) return []
	const data = (json as Record<string, unknown>).data
	if (!Array.isArray(data)) return []
	const items: ResultItem[] = []
	for (const item of data) {
		if (typeof item !== 'object' || item === null) continue
		const entry = item as Record<string, unknown>
		if (typeof entry.b64_json === 'string' && entry.b64_json) items.push({ b64: entry.b64_json })
		else if (typeof entry.url === 'string' && entry.url) items.push({ url: entry.url })
	}
	return items
}

export function bytesToBase64(bytes: Uint8Array): string {
	let binary = ''
	const chunk = 0x8000
	for (let i = 0; i < bytes.length; i += chunk) {
		binary += String.fromCharCode(...bytes.subarray(i, i + chunk))
	}
	return btoa(binary)
}

export function base64ToBytes(b64: string): Uint8Array<ArrayBuffer> {
	const binary = atob(b64)
	const bytes = new Uint8Array(binary.length)
	for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
	return bytes
}

/** 结果图片统一转 data URL（按魔数嗅探格式） */
export function bytesToDataUrl(bytes: Uint8Array): string {
	return `data:${imageMime(sniffExt(bytes))};base64,${bytesToBase64(bytes)}`
}

/** 解析 base64 data URL 参考图；非图片类型或格式坏返回 null */
export function parseDataImageUrl(url: string): InputImage | null {
	const comma = url.indexOf(',', 5)
	if (comma < 0) return null
	const meta = url.slice(5, comma)
	if (!meta.endsWith(';base64')) return null
	const extByMime: Record<string, string> = { 'image/png': 'png', 'image/jpeg': 'jpg', 'image/webp': 'webp', 'image/gif': 'gif' }
	const ext = extByMime[meta.slice(0, -';base64'.length)]
	if (!ext) return null
	try {
		const binary = atob(url.slice(comma + 1))
		const bytes = new Uint8Array(binary.length)
		for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i)
		return { bytes, ext }
	} catch {
		return null
	}
}
