import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
	bytesToDataUrl,
	collectResultItems,
	editsFields,
	extractErrorMessage,
	generationsBody,
	imagePartName,
	nonJsonError,
	parseDataImageUrl,
	parseUsage,
} from './protocol.ts'
import { mergedProfile } from './profiles.ts'

test('generationsBody 组装 JSON 请求体：model、prompt 与映射后的参数', () => {
	const profile = mergedProfile('gpt-image-2', null)
	const body = generationsBody('gpt-image-2', 'a cat', { quality: { t: 'text', v: 'high' }, n: { t: 'number', v: 2 } }, profile)
	assert.deepEqual(body, { model: 'gpt-image-2', prompt: 'a cat', n: 2, quality: 'high' })
})

test('editsFields 组装 multipart 文本字段，数字转字符串', () => {
	const profile = mergedProfile('gpt-image-2', null)
	const fields = editsFields('gpt-image-2', 'a cat', { quality: { t: 'text', v: 'high' }, n: { t: 'number', v: 2 }, ratio: { t: 'number', v: 1.5 } }, profile)
	assert.deepEqual(fields, [
		['model', 'gpt-image-2'],
		['prompt', 'a cat'],
		['n', '2'],
		['quality', 'high'],
		['ratio', '1.5'],
	])
})

test('multipart 图片字段名：单图 image，多图 image[]', () => {
	assert.equal(imagePartName(1), 'image')
	assert.equal(imagePartName(3), 'image[]')
})

test('parseUsage 提取 snake_case 用量（含 image_tokens_details）', () => {
	assert.deepEqual(
		parseUsage({ usage: { input_tokens: 10, output_tokens: 20, total_tokens: 30, input_tokens_details: { image_tokens: 5 } } }),
		{ input_tokens: 10, output_tokens: 20, total_tokens: 30, image_tokens: 5 },
	)
	assert.deepEqual(parseUsage({ usage: { total_tokens: 3 } }), { input_tokens: undefined, output_tokens: undefined, total_tokens: 3, image_tokens: undefined })
	assert.equal(parseUsage({ data: [] }), undefined)
	assert.equal(parseUsage({ usage: null }), undefined)
})

test('extractErrorMessage 优先 error.message，否则退回状态码', () => {
	assert.equal(extractErrorMessage({ error: { message: '上游错误' } }, 502), '上游错误')
	assert.equal(extractErrorMessage({ error: {} }, 502), 'HTTP 502')
	assert.equal(extractErrorMessage(null, 429), 'HTTP 429')
})

test('nonJsonError 截断到 300 字符', () => {
	const long = 'x'.repeat(400)
	const message = nonJsonError(502, long)
	assert.ok(message.startsWith('HTTP 502 · 响应不是 JSON：'))
	assert.equal(Array.from(message).length, 'HTTP 502 · 响应不是 JSON：'.length + 300)
})

test('collectResultItems 收集 b64_json 与 url，忽略无图条目', () => {
	const items = collectResultItems({ data: [{ b64_json: 'AAA' }, { url: 'https://img/x.png' }, { revised_prompt: 'x' }, null] })
	assert.deepEqual(items, [{ b64: 'AAA' }, { url: 'https://img/x.png' }])
	assert.deepEqual(collectResultItems({}), [])
	assert.deepEqual(collectResultItems({ data: 'bad' }), [])
})

test('bytesToDataUrl 按魔数嗅探格式；与 parseDataImageUrl 互逆', () => {
	const jpeg = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 1, 2, 3])
	const dataUrl = bytesToDataUrl(jpeg)
	assert.ok(dataUrl.startsWith('data:image/jpeg;base64,'))
	const parsed = parseDataImageUrl(dataUrl)
	assert.ok(parsed)
	assert.equal(parsed.ext, 'jpg')
	assert.deepEqual(parsed.bytes, jpeg)

	const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0])
	assert.ok(bytesToDataUrl(png).startsWith('data:image/png;base64,'))
})

test('parseDataImageUrl 拒绝非 base64、非图片类型与坏 base64', () => {
	assert.equal(parseDataImageUrl('data:image/png,raw'), null)
	assert.equal(parseDataImageUrl('data:image/svg+xml;base64,PHN2Zw=='), null)
	assert.equal(parseDataImageUrl('data:image/png;base64,!!!'), null)
	assert.equal(parseDataImageUrl('https://example.com/x.png'), null)
})

test('bytesToBase64 处理超过一个分块的输入', () => {
	const bytes = new Uint8Array(0x8000 + 10).map((_, i) => i % 256)
	const parsed = parseDataImageUrl(bytesToDataUrl(bytes))
	assert.ok(parsed)
	assert.deepEqual(parsed.bytes, bytes)
})
