import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
	builtinProfile,
	coerceParamValue,
	deepMerge,
	findParam,
	mergedProfile,
	paramsToBody,
	toRequestValue,
	validateRequest,
	withDefaults,
	type ParamMap,
} from './profiles.ts'

const value = (v: ParamMap[string]) => v

test('内置档案按模型前缀分档', () => {
	const p25 = builtinProfile('gpt-image-2.5-sunburst')
	assert.deepEqual(findParam(p25, 'quality')?.options, ['auto', 'high', 'medium', 'low', 'xhigh', 'max'])
	assert.equal(p25.size_ratios.length, 7)
	assert.ok(p25.size_rule)

	const p2 = builtinProfile('gpt-image-2')
	assert.deepEqual(findParam(p2, 'quality')?.options, ['auto', 'high', 'medium', 'low'])
	assert.equal(p2.size_rule?.step, 16)

	const p1 = builtinProfile('gpt-image-1')
	assert.equal(p1.size_rule, null)
	assert.equal(p1.api, 'openai_images')

	const unknown = builtinProfile('seeddream-v4')
	assert.equal(unknown.api, 'generic')
	assert.equal(unknown.stream, false)
	assert.equal(unknown.params.length, 5)
	assert.equal(unknown.size_rule, null)
})

test('withDefaults 补齐全部协议默认参数，显式值优先（对齐 Rust 行为）', () => {
	const profile = mergedProfile('gpt-image-2', null)
	const full = withDefaults(profile, {})
	for (const def of profile.params) {
		if (def.default_value && def.default_value.t !== 'unset') {
			assert.ok(def.key in full, `默认值未覆盖 ${def.key}`)
		}
	}
	assert.deepEqual(value(full.quality), { t: 'text', v: 'auto' })
	assert.deepEqual(value(full.size), { t: 'text', v: 'auto' })
	assert.deepEqual(value(full.background), { t: 'text', v: 'auto' })
	assert.deepEqual(value(full.output_format), { t: 'text', v: 'png' })
	assert.deepEqual(value(full.input_fidelity), { t: 'text', v: 'low' })
	assert.deepEqual(value(full.n), { t: 'number', v: 1 })
	assert.deepEqual(value(full.output_compression), { t: 'number', v: 100 })
	assert.ok(!('user' in full))

	const explicit = withDefaults(profile, { quality: { t: 'text', v: 'high' } })
	assert.deepEqual(value(explicit.quality), { t: 'text', v: 'high' })
})

test('deepMerge 对象递归、null 删键、数组与标量整体替换', () => {
	const merged = deepMerge({ a: { b: 1, c: 2 }, list: [1, 2], keep: 'x' }, { a: { b: 9, c: null }, list: [3] })
	assert.deepEqual(merged, { a: { b: 9 }, list: [3], keep: 'x' })
	assert.deepEqual(deepMerge({ a: 1 }, { a: { n: 1 } }), { a: { n: 1 } })
})

test('mergedProfile 合并 override；结构损坏时回退内置档案', () => {
	const withOverride = mergedProfile('gpt-image-2', { max_refs: 3, label: '自定义' })
	assert.equal(withOverride.max_refs, 3)
	assert.equal(withOverride.label, '自定义')
	assert.equal(withOverride.params.length, builtinProfile('gpt-image-2').params.length)

	// null 删掉有默认值的字段 → 回落为默认空串
	const removed = mergedProfile('gpt-image-2', { edit_note: null })
	assert.equal(removed.edit_note, '')

	const broken = mergedProfile('gpt-image-2', { params: '不是数组' })
	assert.equal(broken.params.length, builtinProfile('gpt-image-2').params.length)

	const badKind = mergedProfile('gpt-image-2', { params: [{ key: 'x', label: 'X', kind: 'bogus' }] })
	assert.equal(badKind.params.length, builtinProfile('gpt-image-2').params.length)

	// override 完整替换 params 数组（协议：数组不做逐项合并）
	const replaced = mergedProfile('gpt-image-2', { params: [{ key: 'q', label: '画质', kind: 'select', options: ['a'], default_value: { t: 'text', v: 'a' } }] })
	assert.equal(replaced.params.length, 1)
	assert.deepEqual(findParam(replaced, 'q')?.options, ['a'])
})

test('validateRequest 校验模型、图片上限、数值范围与枚举取值', () => {
	const profile = mergedProfile('gpt-image-2', null)
	const base: ParamMap = { quality: { t: 'text', v: 'auto' }, n: { t: 'number', v: 1 } }
	assert.equal(validateRequest(profile, 'gpt-image-2', base, 0), null)
	assert.equal(validateRequest(profile, '', base, 0), '请先选择模型')
	assert.equal(validateRequest(profile, 'gpt-image-2', { ...base, n: { t: 'number', v: 11 } }, 0), '「数量」超出范围')
	assert.equal(validateRequest(profile, 'gpt-image-2', { ...base, quality: { t: 'text', v: 'ultra' } }, 0), '「画质」的取值 ultra 不在可选列表')
	assert.equal(validateRequest(profile, 'gpt-image-2', base, 17), '图片总数超过上限 16 张')
	// 档案外的键是开集合，不校验
	assert.equal(validateRequest(profile, 'gpt-image-2', { ...base, mystery: { t: 'text', v: 'whatever' } }, 0), null)
})

test('coerceParamValue 按 /api/generate 的规则收窄标量', () => {
	assert.deepEqual(coerceParamValue('hi'), { t: 'text', v: 'hi' })
	assert.deepEqual(coerceParamValue(3), { t: 'number', v: 3 })
	assert.deepEqual(coerceParamValue(true), { t: 'text', v: 'true' })
	assert.equal(coerceParamValue({ a: 1 }), null)
	assert.equal(coerceParamValue([1]), null)
	assert.equal(coerceParamValue(null), null)
})

test('paramsToBody 跳过 unset、按 api_key 映射、未知键透传', () => {
	const profile = mergedProfile('gpt-image-2', {
		params: builtinProfile('gpt-image-2').params.map((d) => (d.key === 'quality' ? { ...d, api_key: 'quality_level' } : d)),
	})
	const body = paramsToBody(profile, {
		quality: { t: 'text', v: 'high' },
		n: { t: 'number', v: 2 },
		ignored: { t: 'unset' },
		mystery: { t: 'text', v: 'pass-through' },
	})
	assert.deepEqual(body, { mystery: 'pass-through', n: 2, quality_level: 'high' })
})

test('toRequestValue：unset → null，text/size 原样，数字保持数值', () => {
	assert.equal(toRequestValue({ t: 'unset' }), null)
	assert.equal(toRequestValue({ t: 'text', v: 'x' }), 'x')
	assert.equal(toRequestValue({ t: 'size', v: '1024x1024' }), '1024x1024')
	assert.equal(toRequestValue({ t: 'number', v: 3 }), 3)
	assert.equal(toRequestValue({ t: 'number', v: 3.5 }), 3.5)
})
