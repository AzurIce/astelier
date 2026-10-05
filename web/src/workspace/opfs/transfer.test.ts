import { test } from 'node:test'
import assert from 'node:assert/strict'
import { graphTake, mergeGroups, planImport, type ImportEntry } from './transfer.ts'

const entry = (path: string, updated_at?: number): ImportEntry => ({
	path,
	bytes: new TextEncoder().encode(JSON.stringify({ id: 'g', updated_at: updated_at ?? 1 })),
})

test('planImport 归类图/库/分组，拒绝穿越、越界与未知顶层', () => {
	const plan = planImport([
		entry('atelier/graphs/abc/graph.json'),
		entry('atelier/graphs/abc/view.json'),
		entry('atelier/graphs/abc/store/猫.png'),
		entry('graphs/def/graph.json'),
		entry('atelier/stores/角色/猫.png'),
		entry('atelier/groups.json'),
		entry('../evil.png'),
		entry('/abs/x.png'),
		entry('atelier/config.json'),
		entry('atelier/runs/legacy.json'),
		entry('atelier/graphs/bad../graph.json'),
		entry('atelier/graphs/onlyview/view.json'),
	])
	assert.deepEqual(plan.graphs.map((g) => g.id).sort(), ['abc', 'def'])
	assert.equal(plan.graphs.find((g) => g.id === 'abc')?.store.length, 1)
	assert.equal(plan.library.length, 1)
	assert.equal(plan.library[0].path, '角色/猫.png')
	assert.ok(plan.groups)
	// config 不导入、runs 拒绝、无 graph.json 的目录不成图、路径穿越拒绝
	assert.deepEqual(plan.rejected.filter((p) => !p.includes('onlyview') && p !== 'atelier/graphs/bad../graph.json').sort(), [
		'../evil.png',
		'/abs/x.png',
		'atelier/config.json',
		'atelier/runs/legacy.json',
	])
	assert.ok(plan.rejected.includes('atelier/graphs/bad../graph.json') || plan.graphs.every((g) => g.id !== 'bad..'))
	assert.ok(!plan.graphs.some((g) => g.id === 'onlyview'))
})

test('graphTake：本地缺失或较旧才导入（last-writer-wins）', () => {
	assert.equal(graphTake(null, 1), true)
	assert.equal(graphTake(5, null), false)
	assert.equal(graphTake(5, 10), true)
	assert.equal(graphTake(10, 5), false)
	assert.equal(graphTake(5, 5), true, '同版本按导入处理（恢复备份）')
})

test('mergeGroups：按 id 并集，冲突保留本地', () => {
	const local = [{ id: 'a', name: '本地A', parent_id: null, created_at: 1 }]
	const imported = [
		{ id: 'a', name: '导入A', parent_id: null, created_at: 9 },
		{ id: 'b', name: '导入B', parent_id: null, created_at: 2 },
	]
	const merged = mergeGroups(local, imported)
	assert.equal(merged.length, 2)
	assert.equal(merged.find((g) => g.id === 'a')?.name, '本地A')
	assert.equal(merged.find((g) => g.id === 'b')?.name, '导入B')
})
