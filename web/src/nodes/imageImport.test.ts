import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createImageImporter, sourceName, type ImageSource, type ImportReport } from './imageImport.ts'
import { IMAGE_DRAG_MIME, readImageDrag, writeImageDrag } from './dragPayload.ts'
import type { ImageRef } from './classes'
import { createHash } from 'node:crypto'
import { sha256Fallback } from './imageHash.ts'

const file = (name: string, marker = 1): ImageSource => ({ kind: 'file', file: new File([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, marker])], name, { type: 'image/png' }) })

test('局域网 HTTP 的内容指纹与 SHA-256 一致（空数据、填充边界、多区块）', () => {
	for (const length of [0, 1, 55, 56, 63, 64, 65, 127, 128, 4096]) {
		const bytes = new Uint8Array(length).map((_, index) => index % 256)
		assert.equal(sha256Fallback(bytes.buffer), createHash('sha256').update(bytes).digest('hex'))
	}
})

function setup(upload?: (gid: string, name: string, blob: Blob) => Promise<{ name: string }>) {
	const images: ImageRef[] = []
	const reports: ImportReport[] = []
	const uploads: string[] = []
	let active = true
	const importer = createImageImporter({
		target: () => ({ graphId: 'test-graph', isActive: () => active, images: () => images, append: (image) => { images.push(image); return true } }),
		upload: upload ?? (async (_gid, name) => { uploads.push(name); return { name } }),
		onProgress: () => {},
		onReport: (report) => { reports.push(report) },
	})
	return { importer, images, uploads, reports, deactivate: () => { active = false } }
}

test('多图拖拽保留顺序和元信息；URI 忽略注释和重复，损坏载荷仍可回退', () => {
	const values = new Map<string, string>()
	const transfer = { setData: (type: string, value: string) => { values.set(type, value) }, getData: (type: string) => values.get(type) ?? '' }
	const images = ['a', 'b'].map((name) => ({ kind: 'store' as const, url: `/store/${name}.png`, file: `${name}.png`, store: '', w: 123 }))
	writeImageDrag(transfer as unknown as DataTransfer, images)
	assert.deepEqual(readImageDrag(transfer), images)
	values.set(IMAGE_DRAG_MIME, '{broken')
	values.set('text/uri-list', '# image list\r\n/store/a.png\r\n/store/b.png\r\n/store/a.png\r\nnot a URL\r\njavascript:alert(1)')
	assert.deepEqual(readImageDrag(transfer).map((image) => image.url), ['/store/a.png', '/store/b.png'])
})

test('连续入队保持批次顺序；单张上传失败不丢掉后面的图片', async () => {
	let release!: () => void
	const gate = new Promise<void>((resolve) => { release = resolve })
	const order: string[] = []
	const s = setup(async (_gid, name) => {
		order.push(name)
		if (name === 'a.png') await gate
		if (name === 'bad.png') throw new Error('模拟上传失败')
		return { name }
	})
	const first = s.importer.enqueue([file('a.png', 1), file('bad.png', 2), file('b.png', 3)])
	const second = s.importer.enqueue([file('c.png', 4)])
	release()
	await Promise.all([first, second])
	assert.deepEqual(order, ['a.png', 'bad.png', 'b.png', 'c.png'])
	assert.deepEqual(s.images.map((image) => image.name), ['a.png', 'b.png', 'c.png'])
	assert.equal(s.reports.length, 1)
	assert.equal(s.reports[0].added, 3)
	assert.equal(s.reports[0].failures[0].name, 'bad.png')
})

test('同内容不同名字只上传一次；同名不同内容保留；非图片不打断导入', async () => {
	const s = setup()
	await s.importer.enqueue([file('same.png', 1), file('renamed.png', 1), { kind: 'file', file: new File(['invalid'], 'bad.txt') }, file('same.png', 2)])
	assert.deepEqual(s.uploads, ['same.png', 'same.png'])
	assert.equal(s.images.length, 2)
	assert.notEqual(s.images[0].hash, s.images[1].hash)
	assert.equal(s.reports[0].skipped, 1)
	assert.equal(s.reports[0].failures.length, 1)
})

test('切图 / 删除后，在途上传不能提交，队列不能继续上传', async () => {
	let release!: () => void
	let started!: () => void
	const begun = new Promise<void>((resolve) => { started = resolve })
	const gate = new Promise<void>((resolve) => { release = resolve })
	let uploads = 0
	const s = setup(async (_gid, name) => { uploads++; started(); await gate; return { name } })
	const pending = s.importer.enqueue([file('a.png', 1), file('b.png', 2)])
	await begun
	s.deactivate()
	release()
	await pending
	assert.equal(uploads, 1)
	assert.equal(s.images.length, 0)
})

test('组件销毁取消队列；上传返回后不会提交或触发 UI 回调', async () => {
	let release!: () => void
	let started!: () => void
	const begun = new Promise<void>((resolve) => { started = resolve })
	const gate = new Promise<void>((resolve) => { release = resolve })
	const s = setup(async (_gid, name) => { started(); await gate; return { name } })
	const pending = s.importer.enqueue([file('a.png', 1), file('b.png', 2)])
	await begun
	s.importer.dispose()
	release()
	await pending
	assert.equal(s.images.length, 0)
	assert.equal(s.reports.length, 0)
})

test('URI 名字移除查询串并解码；坏百分号不会使批量导入崩溃', () => {
	assert.equal(sourceName({ kind: 'url', image: { kind: 'store', url: 'https://example.test/%E7%8C%AB.png?token=123', store: '', file: '' } }), '猫.png')
	assert.equal(sourceName({ kind: 'url', image: { kind: 'store', url: '/store/bad%name.png', store: '', file: '' } }), 'bad%name.png')
	assert.equal(sourceName({ kind: 'url', image: { kind: 'store', url: 'data:image/png;base64,AAAA', store: '', file: '' } }), 'image.png')
})
