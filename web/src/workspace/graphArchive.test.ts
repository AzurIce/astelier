import { test } from 'node:test'
import assert from 'node:assert/strict'
import { zipSync, unzipSync } from 'fflate'
import { createGraphArchive, importGraphArchive, parseGraphArchive } from './graphArchive'
import type { WorkspaceStore } from './store'

const encode = (value: unknown) => new TextEncoder().encode(JSON.stringify(value))
const graph = { id: 'original', title: '示例/图', nodes: [{ id: 'image', type: 'image', params: { images: [{ file: '参考.jpeg', name: '参考图' }] } }], edges: [], created_at: 1, updated_at: 2 }
const view = { positions: { image: { x: 20, y: 30 } }, viewport: { x: 4, y: 5, zoom: 0.8 } }
const png = new Uint8Array([137, 80, 78, 71])
const fixture = () => ({ 'graph.json': encode(graph), 'view.json': encode(view), 'store/参考.jpeg': png, 'store/未引用.png': png })

test('single graph ZIP roundtrip retains layout and all assets, creates a copy and remaps normalized filenames', async () => {
	const calls: unknown[] = []
	const source = {
		fetchGraph: async () => graph,
		fetchView: async () => view,
		listGraphStoreFiles: async () => [{ name: '参考.jpeg' }, { name: '未引用.png' }],
		graphStoreUrl: async () => 'data:image/png;base64,iVBORw==',
	} as unknown as WorkspaceStore
	const file = await createGraphArchive(source, 'original')
	assert.equal(file.name, '示例-图.astelier')
	const entries = unzipSync(new Uint8Array(await file.arrayBuffer()))
	assert.deepEqual(Object.keys(entries).sort(), ['graph.json', 'store/', 'store/参考.jpeg', 'store/未引用.png', 'view.json'].sort())
	assert.equal(JSON.parse(new TextDecoder().decode(entries['graph.json'])).updated_at, 2)
	const target = {
		createGraph: async (group: string | null, title: string) => { calls.push([group, title]); return { id: 'new-id', group_id: group, title } },
		uploadGraphStoreFile: async (_id: string, name: string) => { calls.push(name); return { name: name.replace('.jpeg', '.jpg') } },
		putGraph: async (id: string, doc: unknown) => calls.push([id, doc]),
		putView: async (id: string, doc: unknown) => calls.push([id, doc]),
	} as unknown as WorkspaceStore
	const imported = await importGraphArchive(target, file, 'destination-folder')
	assert.equal(imported.id, 'new-id')
	assert.equal(imported.group_id, 'destination-folder')
	assert.deepEqual(imported.nodes[0].params.images, [{ file: '参考.jpg', name: '参考图' }])
	assert(calls.includes('未引用.png'))
	assert.deepEqual(calls[0], ['destination-folder', '示例/图'])
})

test('invalid archives, traversal, missing references, duplicate nodes and invalid layouts fail before writes', async () => {
	for (const entries of [
		{ ...fixture(), '../config.json': encode({}) },
		{ ...fixture(), 'store/../逃逸.png': png },
		{ ...fixture(), 'store/参考.jpeg': new Uint8Array() },
		{ ...fixture(), 'graph.json': encode({ ...graph, nodes: [...graph.nodes, graph.nodes[0]] }) },
		{ ...fixture(), 'view.json': encode({ positions: { image: { x: 'bad', y: 0 } } }) },
		{ 'graph.json': encode(graph) },
	]) {
		let created = false
		const target = { createGraph: async () => { created = true } } as unknown as WorkspaceStore
		await assert.rejects(() => importGraphArchive(target, new File([zipSync(entries)], 'bad.astelier')))
		assert.equal(created, false)
	}
	assert.throws(() => parseGraphArchive(encode({})), /ZIP/)
})

test('failed asset upload removes the incomplete new graph and preserves the source error', async () => {
	const deleted: string[] = []
	const target = {
		createGraph: async () => ({ id: 'partial' }),
		uploadGraphStoreFile: async () => { throw new Error('disk full') },
		deleteGraph: async (id: string) => { deleted.push(id) },
	} as unknown as WorkspaceStore
	await assert.rejects(() => importGraphArchive(target, new File([zipSync(fixture())], 'example.astelier')), /disk full/)
	assert.deepEqual(deleted, ['partial'])
})

test('legacy view outputs are discarded from imported packages', () => {
	const archive = parseGraphArchive(zipSync({ ...fixture(), 'view.json': encode({ ...view, outputs: { image: 'data:image/png;base64,abc' } }) }))
	assert.equal('outputs' in archive.view, false)
})
