import { test } from 'node:test'
import assert from 'node:assert/strict'
import { OPENAI_IMAGE_PARAMS } from './params.ts'

test('参数表由内置档案派生：默认值齐全、控件形态与尺寸候选保留', () => {
	const byKey = Object.fromEntries(OPENAI_IMAGE_PARAMS.map((p) => [p.key, p]))
	// 与协议默认一致（profiles.ts 是唯一事实来源）
	assert.equal(byKey.quality.def, 'auto')
	assert.equal(byKey.size.def, 'auto')
	assert.equal(byKey.n.def, 1)
	assert.equal(byKey.background.def, 'auto')
	assert.equal(byKey.output_format.def, 'png')
	assert.equal(byKey.input_fidelity.def, 'low')
	assert.equal(byKey.output_compression.def, 100)
	assert.equal(byKey.moderation.def, 'auto')
	// UI 展示扩展
	assert.equal(byKey.quality.control, 'slider')
	assert.equal(byKey.background.control, 'segmented')
	assert.deepEqual(
		byKey.size.options,
		['auto', '1024x1024', '1536x1024', '1024x1536', '1792x1008', '1008x1792'],
	)
	// 归一化后每个参数都有可发送的具体默认（「始终完整发送」）
	for (const p of OPENAI_IMAGE_PARAMS) {
		assert.ok(p.def !== null && p.def !== '', `${p.key} 需要具体默认值`)
	}
	// 数值参数范围来自档案
	assert.equal(byKey.n.min, 1)
	assert.equal(byKey.n.max, 10)
})
