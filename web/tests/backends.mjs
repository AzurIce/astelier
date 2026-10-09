// Real OPFS + two real Rust backends + a free, controlled provider endpoint.
// No external provider is contacted. Server keys come from per-process env vars.
import assert from 'node:assert/strict'
import { createServer } from 'node:http'
import { spawn } from 'node:child_process'
import { mkdtemp, mkdir, writeFile, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { pathToFileURL, fileURLToPath } from 'node:url'
import { unzipSync } from 'fflate'
import { randomUUID } from 'node:crypto'
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright')
const PNG = 'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a0S8AAAAASUVORK5CYII='
const png = `data:image/png;base64,${PNG}`
const requests = [], children = [], roots = [], errors = [], apiRequests = []
let heldResponse = null, hold = false
const upstream = createServer(async (req, res) => {
  res.setHeader('Access-Control-Allow-Origin', '*')
  res.setHeader('Access-Control-Allow-Headers', 'Authorization, Content-Type')
  res.setHeader('Access-Control-Allow-Methods', 'POST, OPTIONS')
  if (req.method === 'OPTIONS') { res.writeHead(204).end(); return }
  const chunks = []; for await (const chunk of req) chunks.push(chunk)
  requests.push({ path: req.url, key: req.headers.authorization, type: req.headers['content-type'], body: Buffer.concat(chunks) })
  const respond = () => { res.setHeader('Content-Type', 'application/json'); res.end(JSON.stringify({ data: [{ b64_json: PNG }, { b64_json: PNG }] })) }
  if (hold) heldResponse = respond; else respond()
})
const listen = (server) => new Promise((done) => server.listen(0, '127.0.0.1', done))
const close = (server) => new Promise((done) => server.close(done))
const until = async (check) => { for (let i = 0; i < 500; i++) { if (await check()) return; await new Promise((r) => setTimeout(r, 10)) } throw new Error('Timed out waiting for controlled server') }
const nodes = (name, backendId = 'local') => [
  { id: 'model', type: 'model', params: { providerBackendId: backendId, provider: 'mock', modelId: 'gpt-image-2' } },
  { id: 'prompt', type: 'prompt', params: { text: name } },
  { id: 'generate', type: 'generate', params: {} },
  { id: 'preview', type: 'preview', params: {} },
]
const edges = [
  { id: 'm-g', source: 'model', sourcePort: 'model', target: 'generate', targetPort: 'model' },
  { id: 'p-g', source: 'prompt', sourcePort: 'text', target: 'generate', targetPort: 'prompt' },
  { id: 'g-p', source: 'generate', sourcePort: 'image', target: 'preview', targetPort: 'image' },
]
let browser
try {
  await listen(upstream)
  const providerUrl = `http://127.0.0.1:${upstream.address().port}/v1`
  for (const name of ['A', 'B']) {
    const root = await mkdtemp(resolve(tmpdir(), 'astelier-backend-')); roots.push(root)
    const socket = createServer(); await listen(socket); const port = socket.address().port; await close(socket)
    const base = `http://127.0.0.1:${port}`
    const backendId = randomUUID().replaceAll('-', '')
    await writeFile(resolve(root, 'backend.json'), JSON.stringify({ id: backendId }))
    await mkdir(resolve(root, 'graphs/shared'), { recursive: true })
    await writeFile(resolve(root, 'graphs/shared/graph.json'), JSON.stringify({ id: 'shared', title: `Server ${name}`, group_id: null, nodes: nodes(name, backendId), edges, created_at: 1, updated_at: 1 }))
    await writeFile(resolve(root, 'config.json'), JSON.stringify({ active_provider: 'mock', providers: [{ id: 'mock', name: `Remote ${name}`, base_url: providerUrl, api_key: 'ASTELIER_TEST_KEY_2', models: ['gpt-image-2'], overrides: { 'gpt-image-2': { label: `Profile ${name}` } } }] }))
    const child = spawn(resolve(process.env.ASTELIER_SERVER_BIN ?? fileURLToPath(new URL('../../target/debug/astelier', import.meta.url))), [], { env: { ...process.env, ASTELIER_ADDR: `127.0.0.1:${port}`, ASTELIER_DATA_DIR: root, ASTELIER_TEST_KEY_2: `sk-server-${name}` }, stdio: ['ignore', 'pipe', 'pipe'] })
    let log = ''; child.stdout.on('data', (chunk) => { log += chunk }); child.stderr.on('data', (chunk) => { log += chunk });
    child.on('error', (error) => { log += error.message }); children.push(child)
    await until(async () => { if (child.exitCode !== null) throw new Error(log); try { return (await fetch(`${base}/api/providers`)).ok } catch { return false } })
    const discovery = await (await fetch(`${base}/api/providers`)).text()
    assert(!discovery.includes('ASTELIER_TEST_KEY_2')); assert(!discovery.includes(`sk-server-${name}`)); assert(JSON.parse(discovery).every((provider) => !('api_key' in provider)))
    roots[roots.length - 1] = { root, base }
  }
  const [a, b] = roots
  browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? 'chromium', headless: true })
  const page = await browser.newPage({ viewport: { width: 1600, height: 1100 }, acceptDownloads: true })
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('request', (req) => { if (new URL(req.url()).pathname.startsWith('/api/')) apiRequests.push(new URL(req.url()).pathname) })
  await page.goto(process.env.ASTELIER_TEST_URL ?? 'http://127.0.0.1:5173')
  await page.waitForFunction(() => document.querySelector('.run-btn') && !document.querySelector('.run-btn').disabled)
  const localId = await page.evaluate(async ({ providerUrl, nodes, edges }) => {
    const registry = await import('/src/backends/registry.svelte.ts')
    const session = await import('/src/canvas/session.svelte.ts')
    await session.flushNow()
    await (await import('/src/generation/localConfig.ts')).saveLocalConfig({ active_provider: 'mock', providers: [{ id: 'mock', name: 'Browser', base_url: providerUrl, api_key: 'sk-browser', models: ['gpt-image-2'], overrides: {} }] })
    await registry.refreshProviders('local')
    const store = registry.backendStore('local')
    const graph = await store.createGraph(null, 'Local graph')
    await store.putGraph(graph.id, { version: 1, nodes, edges })
    await session.openGraph({ backendId: 'local', id: graph.id })
    return graph.id
  }, { providerUrl, nodes: nodes('Local'), edges })

  // Add A through the actual connection UI, B through the same underlying operation.
  await page.getByRole('button', { name: '添加 / 管理', exact: true }).click()
  await page.locator('.connection-add').getByRole('textbox', { name: '名称', exact: true }).fill('Server A')
  await page.locator('.connection-add').getByRole('textbox', { name: '服务地址', exact: true }).fill(a.base)
  await page.getByRole('button', { name: '添加服务器', exact: true }).click()
  await page.waitForFunction(() => [...document.querySelectorAll('.connection')].some((el) => el.textContent.includes('Remote A')))
  await page.getByRole('button', { name: '关闭', exact: true }).click()
  const ids = await page.evaluate(async (base) => {
    const r = await import('/src/backends/registry.svelte.ts')
    const a = r.backendRegistry.entries.find((entry) => entry.name === 'Server A').id
    const b = await r.addServer('Server B', base)
    return { a, b }
  }, b.base)
  assert.equal(await page.locator('.backend-root').count(), 3)
  assert.equal(await page.locator('.dock .tree-row').filter({ hasText: 'Server A' }).count(), 1)
  await page.locator('.ui-node[data-node-id="model"] select[aria-label="Provider"]').selectOption(JSON.stringify([ids.a, 'mock']))
  await page.locator('.run-btn').click(); await page.waitForSelector('.result-img')
  assert.equal(requests.at(-1).key, 'Bearer sk-server-A')
  assert.equal(requests.at(-1).path, '/v1/images/generations')
  console.log('PASS local graph uses Remote Provider A; server resolves its environment key')

  await page.evaluate(async (backendId) => (await import('/src/canvas/session.svelte.ts')).openGraph({ backendId, id: 'shared' }), ids.b)
  await page.locator('.ui-node[data-node-id="model"] select[aria-label="Provider"]').selectOption(JSON.stringify(['local', 'mock']))
  await page.locator('.run-btn').click(); await page.waitForSelector('.result-img')
  assert.equal(requests.at(-1).key, 'Bearer sk-browser')
  await page.evaluate(async () => { const { rt } = await import('/src/canvas/runtime.ts'); const session = await import('/src/canvas/session.svelte.ts'); rt.editor.getNode('prompt').text = 'Edited B'; session.scheduleSave(); await session.flushNow() })
  assert.equal((await (await fetch(`${b.base}/api/graphs/shared`)).json()).nodes.find((n) => n.id === 'prompt').params.text, 'Edited B')
  assert.equal((await (await fetch(`${a.base}/api/graphs/shared`)).json()).nodes.find((n) => n.id === 'prompt').params.text, 'A')
  console.log('PASS server graph uses Local Provider; same graph/node/provider IDs stay isolated by backend')

  // Cross-backend library copies and persistent OPFS blob references.
  await page.evaluate(async ({ ids, png, localId }) => {
    const r = await import('/src/backends/registry.svelte.ts')
    const bytes = Uint8Array.from(atob(png.split(',')[1]), (c) => c.charCodeAt(0))
    await r.backendStore(ids.b).uploadStoreFile(new File([bytes], 'reference.png', { type: 'image/png' }), '')
    await (await import('/src/library/transfer.ts')).copyStorePaths(ids.b, ['reference.png'], 'local', '共享')
    await (await import('/src/library/transfer.ts')).copyStorePaths('local', ['共享'], ids.a, '收藏')
    const session = await import('/src/canvas/session.svelte.ts'); await session.openGraph({ backendId: 'local', id: localId })
    const { rt } = await import('/src/canvas/runtime.ts')
    const { LoadImageNode } = await import('/src/canvas/nodes/model.svelte.ts')
    const image = new LoadImageNode(); image.id = 'reference'; await rt.editor.addNode(image)
  }, { ids, png, localId })
  const libraryReference = await page.evaluate(async () => {
    const r = await import('/src/backends/registry.svelte.ts')
    return { kind: 'store', backendId: 'local', store: '', file: 'reference.png', url: await r.backendStore('local').storeUrl('共享/reference.png') }
  })
  await page.locator('.ui-node[data-node-id="reference"] .image-editor').evaluate((el, image) => { const transfer = new DataTransfer(); transfer.setData('application/x-astelier-images', JSON.stringify([image])); el.dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: transfer })) }, libraryReference)
  await page.waitForFunction(async () => { const { rt } = await import('/src/canvas/runtime.ts'); return rt.editor.getNode('reference').images.length === 1 })
  const reference = await page.evaluate(async ({ backendId }) => {
    const { rt } = await import('/src/canvas/runtime.ts'); const { connect } = await import('/src/canvas/nodes/types.ts')
    const image = rt.editor.getNode('reference'); await rt.editor.addConnection(connect(image, 'image', rt.editor.getNode('generate'), 'image'))
    const model = rt.editor.getNode('model'); model.providerBackendId = backendId; model.provider = 'mock'
    const session = await import('/src/canvas/session.svelte.ts'); session.scheduleSave(); await session.flushNow()
    return image.images[0]
  }, { backendId: ids.a })
  assert(!reference.dataUrl, 'OPFS library blob references must copy into the graph')
  await page.locator('.run-btn').click(); await page.waitForSelector('.result-img')
  assert.equal(requests.at(-1).path, '/v1/images/edits')
  assert(requests.at(-1).type.startsWith('multipart/form-data'))
  assert(requests.at(-1).body.includes(Buffer.from(PNG, 'base64')))
  assert((await (await fetch(`${a.base}/api/stores`)).json()).files.some((f) => f.path === '收藏/共享/reference.png'))
  console.log('PASS cross-backend image/library copies and browser references reach server multipart edits')

  hold = true; heldResponse = null
  await page.evaluate(async () => { window.pendingRun = (await import('/src/canvas/execute.ts')).runPipeline().then(() => true, () => false) })
  await until(() => heldResponse !== null)
  await page.evaluate(async ({ ids, localId }) => { const session = await import('/src/canvas/session.svelte.ts'); await session.openGraph({ backendId: ids.a, id: 'shared' }); await session.openGraph({ backendId: 'local', id: localId }) }, { ids, localId })
  heldResponse(); heldResponse = null; hold = false
  assert.equal(await page.evaluate(() => window.pendingRun), false)
  assert.equal(await page.locator('.result-img').count(), 0)
  console.log('PASS returning to the same graph does not accept an older session generation result')

  // Actual export/download/import with empty folders, references, View and library.
  await page.evaluate(async () => { const r = await import('/src/backends/registry.svelte.ts'); await r.backendStore('local').makeStoreDir('空目录'); await (await import('/src/canvas/session.svelte.ts')).flushNow() })
  const downloadPending = page.waitForEvent('download')
  await page.evaluate(async () => (await import('/src/workspace/opfs/transfer.ts')).exportWorkspaceZip())
  const stream = await (await downloadPending).createReadStream(); const chunks = []; for await (const chunk of stream) chunks.push(chunk)
  const zipped = Buffer.concat(chunks), entries = unzipSync(zipped)
  assert(!entries['astelier/config.json']); assert(!zipped.includes(Buffer.from('sk-browser')))
  assert(entries[`astelier/graphs/${localId}/store/${reference.file}`]); assert(entries['astelier/stores/空目录/'])
  const imported = await browser.newPage()
  imported.on('pageerror', (error) => errors.push(error.message))
  await imported.goto(process.env.ASTELIER_TEST_URL ?? 'http://127.0.0.1:5173'); await imported.waitForSelector('.ui-node')
  const restored = await imported.evaluate(async ({ zip, localId, referenceFile }) => {
    const session = await import('/src/canvas/session.svelte.ts'); await session.flushNow()
    const fs = await import('/src/workspace/opfs/fs.ts'); await (await navigator.storage.getDirectory()).removeEntry('astelier', { recursive: true })
    const report = await (await import('/src/workspace/opfs/transfer.ts')).importWorkspaceZip(new File([Uint8Array.from(zip)], 'backup.zip'))
    const r = await import('/src/backends/registry.svelte.ts'); const store = r.backendStore('local')
    return { report, graph: await store.fetchGraph(localId), view: await store.fetchView(localId), tree: await store.storeTree(), reference: await store.graphStoreUrl(localId, referenceFile), configAbsent: !(await fs.readJson([...fs.WORKSPACE_ROOT, 'config.json'])) }
  }, { zip: [...zipped], localId, referenceFile: reference.file })
  assert(restored.report.graphsTaken >= 1); assert(restored.reference?.startsWith('blob:')); assert(restored.tree.dirs.includes('空目录')); assert(restored.tree.files.some((f) => f.path === '共享/reference.png')); assert(restored.configAbsent); assert(restored.view.positions)
  await imported.close()
  console.log('PASS OPFS zip roundtrip restores graph/view/reference/library/empty folders and excludes keys')

  await page.reload(); await page.waitForFunction(() => document.querySelector('.run-btn') && !document.querySelector('.run-btn').disabled)
  await page.waitForFunction(async () => (await import('/src/backends/registry.svelte.ts')).backendRegistry.entries.every((e) => e.status === 'online'))
  assert.equal(await page.locator('.backend-root').count(), 3)
  assert.equal(await page.locator('.result-img').count(), 0)
  await page.evaluate(async (id) => (await import('/src/canvas/session.svelte.ts')).openGraph({ backendId: id, id: 'shared' }), ids.a)
  await page.screenshot({ path: process.env.ASTELIER_TEST_SCREENSHOT ?? '/tmp/astelier-multiple-backends.png' })
  await page.evaluate(async (id) => (await import('/src/canvas/session.svelte.ts')).detachBackend(id), ids.a)
  assert.equal(await page.locator('.backend-root').count(), 2)
  assert.equal((await (await fetch(`${a.base}/api/graphs/shared`)).json()).id, 'shared')
  assert.equal(await page.evaluate(async () => (await import('/src/canvas/session.svelte.ts')).graphSession.backendId), 'local')
  children[1].kill('SIGTERM'); await new Promise((done) => children[1].once('exit', done))
  await page.evaluate(async (id) => (await import('/src/backends/registry.svelte.ts')).connectBackend(id), ids.b)
  assert.equal(await page.evaluate(async (id) => (await import('/src/backends/registry.svelte.ts')).backend(id).status, ids.b), 'offline')
  assert.equal(await page.locator('.ui-node').count(), 5)
  assert(!apiRequests.some((path) => path === '/api/config'))
  assert.deepEqual(errors, [])
  console.log('PASS connections restore; unmount keeps remote data; offline backend leaves local graph usable; no key-fetch requests or browser errors')
} finally {
  if (heldResponse) heldResponse()
  if (browser) await browser.close()
  for (const child of children) if (child.exitCode === null) child.kill('SIGTERM')
  await close(upstream)
  for (const item of roots) await rm(typeof item === 'string' ? item : item.root, { recursive: true, force: true })
}
