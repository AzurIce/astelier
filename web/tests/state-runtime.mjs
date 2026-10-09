// Browser regression: real Svelte/Rete rendering against the real OPFS workspace.
// The generator is stubbed and save failures are injected via window.__astelierRuntime
// (see main.ts); no provider/key is used and no /api request is made.
import assert from 'node:assert/strict'
import { pathToFileURL } from 'node:url'
import { resolve } from 'node:path'
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright')

const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a0S8AAAAASUVORK5CYII='
const fixtureNodes = () => ([
  { id: 'model', type: 'model', params: { providerBackendId: 'local', provider: 'mock', modelId: 'gpt-image-2' } },
  { id: 'prompt', type: 'prompt', params: { text: 'Original prompt' } },
  { id: 'generate', type: 'generate', params: { n: 2, quality: 'low' } },
  { id: 'preview', type: 'preview', params: {} },
])
const fixtureEdges = () => ([
  { id: 'm-g', source: 'model', sourcePort: 'model', target: 'generate', targetPort: 'model' },
  { id: 'p-g', source: 'prompt', sourcePort: 'text', target: 'generate', targetPort: 'prompt' },
  { id: 'g-p', source: 'generate', sourcePort: 'image', target: 'preview', targetPort: 'image' },
])

const pageErrors = []
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? 'chromium', headless: true })
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } })
page.on('pageerror', (error) => pageErrors.push(error.message))

// 注入缝：替身生图器（记录快照、可挂起、可报错）与保存故障包装。
await page.addInitScript(({ png }) => {
  window.__png = png
  window.__generateCalls = []
  window.__astelierRuntime = {
    generate: async (params) => {
      window.__generateCalls.push(JSON.parse(JSON.stringify(params)))
      if (window.__generateGate) await window.__generateGate.promise
      if (window.__generateMode === 'error') throw new Error('Mock upstream failure')
      return { imageUrls: [window.__png, window.__png] }
    },
    wrapStore: (store) => ({
      ...store,
      putGraph: async (id, doc) => {
        if (window.__saveGate) await window.__saveGate.promise
        if (window.__failSave) throw new Error('Mock disk failure')
        return store.putGraph(id, doc)
      },
    }),
  }
}, { png })

const evaluate = (fn, args) => page.evaluate(fn, args)
const gateOn = (name) => evaluate((name) => { let release; const promise = new Promise((r) => { release = r }); window[name] = { promise, release } }, name)
const gateOff = async (name) => { await evaluate((name) => { window[name]?.release(); window[name] = null }, name) }
const setFlag = (name, value) => evaluate(([n, v]) => { window[n] = v }, [name, value])

const loadModules = async () => evaluate(async () => {
  const { rt } = await import('/src/canvas/runtime.ts')
  const store = await import('/src/canvas/session.svelte.ts')
  await store.flushNow()
  window.testRt = rt; window.testStore = store
  window.nodeUpdates = 0
  const originalUpdate = rt.area.update.bind(rt.area)
  rt.area.update = (...args) => { if (args[0] === 'node') window.nodeUpdates++; return originalUpdate(...args) }
})

try {
  // 首次加载：应用在空白 OPFS 上自动建种子图；随后种入带固定节点 id 的 A/B
  // 两张图（A 的视图带 legacy outputs），删掉种子图后刷新加载 A。
  await page.goto(process.env.ASTELIER_TEST_URL ?? 'http://127.0.0.1:5173')
  await page.waitForSelector('.ui-node')
  const ids = await evaluate(async ({ png, nodes, edges }) => {
    const api = (await import('/src/backends/registry.svelte.ts')).backendStore('local')
    const fs = await import('/src/workspace/opfs/fs.ts')
    const seedId = JSON.parse(localStorage.getItem('astelier-active-graph') ?? 'null')?.id
    await (await import('/src/generation/localConfig.ts')).saveLocalConfig({ active_provider: 'mock', providers: [{ id: 'mock', name: 'Mock', models: ['gpt-image-2'], base_url: 'https://mock.example/v1', api_key: 'sk-fake', overrides: {} }] })
    const idA = (await api.createGraph(null, 'Graph A')).id
    const idB = (await api.createGraph(null, 'Graph B')).id
    await api.putGraph(idA, { version: 1, nodes, edges })
    await api.putGraph(idB, { version: 1, nodes, edges })
    // 旧版本会把生成结果缓存进 view.outputs —— 加载时必须忽略
    await fs.writeJson([...fs.WORKSPACE_ROOT, 'graphs', idA, 'view.json'], {
      version: 1,
      positions: { model: { x: 30, y: 10 }, prompt: { x: 30, y: 190 }, generate: { x: 420, y: 10 }, preview: { x: 790, y: 10 } },
      outputs: { generate: png, preview: png },
    })
    if (seedId && seedId !== idA && seedId !== idB) await api.deleteGraph(seedId)
    localStorage.setItem('astelier-active-graph', JSON.stringify({ backendId: 'local', id: idA }))
    return { idA, idB }
  }, { png, nodes: fixtureNodes(), edges: fixtureEdges() })

  await page.reload()
  await page.waitForSelector('.ui-node[data-node-id="generate"]')
  await page.waitForFunction(() => !document.querySelector('.run-btn').disabled)
  await loadModules()
  assert.equal(await page.locator('.result-img').count(), 0, 'legacy View outputs must not restore results')
  console.log('PASS legacy view outputs ignored; graph restored from OPFS with stable node ids')

  await evaluate(() => { window.testRt.editor.getNode('prompt').text = 'Reactive prompt'; window.testRt.editor.getNode('generate').params.n = 3 })
  await page.waitForFunction(() => document.querySelector('textarea').value === 'Reactive prompt')
  assert.equal(await evaluate(() => window.nodeUpdates), 0)
  console.log('PASS reactive node fields update real UI without area.update')

  await gateOn('__generateGate')
  await page.locator('.run-btn').click()
  await page.waitForFunction(() => document.querySelector('[data-node-id="generate"]').classList.contains('busy'))
  const captured = await evaluate(() => window.__generateCalls[0])
  assert.equal(captured.prompt, 'Reactive prompt')
  assert.equal(captured.params.n, 3)
  await evaluate(() => { window.testRt.editor.getNode('generate').params.n = 9 })
  assert.equal(captured.params.n, 3, 'request params must be snapshotted at submit')
  await gateOff('__generateGate')
  await page.waitForSelector('.result-img')
  assert.equal(await page.locator('.result-img').count(), 2)
  assert.equal(await evaluate(() => window.nodeUpdates), 0)
  assert.equal((await evaluate(async () => (await import('/src/backends/registry.svelte.ts')).backendStore('local').storeTree())).files.length, 0)
  console.log('PASS busy and outputs react automatically; request snapshot stable; no automatic collection')

  await page.waitForFunction(() => {
    const path = document.querySelector('.ui-conn.t-image .wire')
    const socket = document.querySelector('[data-node-id="generate"] .port-row.output .ui-socket')
    if (!path || !socket) return false
    const numbers = path.getAttribute('d').match(/-?\d+(?:\.\d+)?/g).map(Number)
    const point = new DOMPoint(numbers[0], numbers[1]).matrixTransform(path.getScreenCTM())
    const rect = socket.getBoundingClientRect()
    return Math.hypot(point.x - rect.left - rect.width / 2, point.y - rect.top - rect.height / 2) < 2
  })
  console.log('PASS connection endpoint follows reactive gallery height changes')

  await evaluate(async () => {
    const { LoadImageNode } = await import('/src/canvas/nodes/model.svelte.ts')
    const node = new LoadImageNode(); node.id = 'image'
    await window.testRt.editor.addNode(node)
    await window.testRt.area.translate(node.id, { x: 30, y: 450 })
  })
  await page.waitForSelector('[data-node-id="image"] .image-editor')
  await evaluate((url) => {
    const transfer = new DataTransfer(); transfer.setData('text/uri-list', url)
    document.querySelector('[data-node-id="image"] .image-editor').dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: transfer }))
  }, png)
  await page.waitForFunction(() => window.testRt.editor.getNode('image').images.length === 1)
  assert.equal((await evaluate(async () => (await import('/src/backends/registry.svelte.ts')).backendStore('local').storeTree())).files.length, 0, 'temporary image must not upload anywhere')
  const serialized = await evaluate(async () => {
    const { toDoc, toViewDoc } = await import('/src/canvas/document.ts')
    await window.testStore.flushNow()
    return { doc: toDoc(), view: toViewDoc() }
  })
  assert.deepEqual(serialized.doc.nodes.find((node) => node.id === 'image').params.images, [])
  assert(!('outputs' in serialized.view))
  assert(!JSON.stringify(serialized).includes('data:image'))
  console.log('PASS temporary image drag creates session reference; Graph/View exclude runtime data')

  await evaluate((url) => {
    const transfer = new DataTransfer(); transfer.setData('text/uri-list', url)
    document.querySelector('.grid-wrap').dispatchEvent(new DragEvent('drop', { bubbles: true, cancelable: true, dataTransfer: transfer }))
  }, png)
  await page.waitForSelector('.entry[data-entry]')
  assert.equal((await evaluate(async () => (await import('/src/backends/registry.svelte.ts')).backendStore('local').storeTree())).files.length, 1)
  console.log('PASS explicit drag into library persists one selected image into OPFS')

  // 库改名/移动：路径即身份，移动后树与旧 URL 缓存立即更新
  const moved = await evaluate(async () => {
    const api = (await import('/src/backends/registry.svelte.ts')).backendStore('local')
    const before = (await api.storeTree()).files[0].path
    const renamed = before.replace(/\.png$/, '') + '-改名.png'
    await api.makeStoreDir('收藏')
    await api.moveStorePath(before, `收藏/${renamed}`)
    const tree = await api.storeTree()
    return { before, tree, after: tree.files[0]?.path }
  })
  assert.equal(moved.after, `收藏/${moved.before.replace(/\.png$/, '')}-改名.png`)
  assert.ok(moved.tree.dirs.includes('收藏'))
  await page.locator('.dock-head [aria-label="刷新"]').click()
  await page.waitForSelector('.entry[data-entry]')
  console.log('PASS library rename and move keep path identity; tree refreshes')

  await evaluate(async (id) => { await window.testStore.openGraph({ backendId: 'local', id }) }, ids.idB)
  assert.equal(await page.locator('.graph-title .name').textContent(), 'Graph B')
  assert.equal(await page.locator('.result-img').count(), 0)
  await gateOn('__saveGate')
  await evaluate(() => { window.testRt.editor.getNode('prompt').text = 'Wait for save'; window.testStore.scheduleSave() })
  await page.waitForFunction(() => window.testStore.graphSession.saveState === 'saving')
  await evaluate(async (id) => { window.switchDone = false; window.switchPromise = window.testStore.openGraph({ backendId: 'local', id }).then(() => { window.switchDone = true }) }, ids.idA)
  assert.equal(await evaluate(() => window.testStore.activeGraph().id), ids.idB)
  assert.equal(await evaluate(() => window.switchDone), false)
  await gateOff('__saveGate')
  await evaluate(() => window.switchPromise)
  assert.equal(await evaluate(() => window.testStore.activeGraph().id), ids.idA)
  assert.equal(await page.locator('.graph-title .name').textContent(), 'Graph A')
  console.log('PASS graph switching awaits already-started writes and shares current title')

  await setFlag('__failSave', true)
  await evaluate(() => { window.testStore.scheduleSave() })
  await page.waitForFunction(() => window.testStore.graphSession.saveState === 'error')
  const blocked = await evaluate(async (id) => { try { await window.testStore.openGraph({ backendId: 'local', id }); return false } catch { return true } }, ids.idB)
  assert(blocked)
  assert.equal(await evaluate(() => window.testStore.activeGraph().id), ids.idA)
  assert.equal(await page.locator('.save-dot').textContent().then((text) => text.trim()), '保存失败')
  await setFlag('__failSave', false)
  await evaluate(() => window.testStore.flushNow())
  console.log('PASS failed save stays visible and blocks switching until retry succeeds')

  // A/B 共用节点 id，验证晚返回结果不能写进切换后同 id 的新节点。
  await gateOn('__generateGate')
  await evaluate(async () => {
    const { runPipeline } = await import('/src/canvas/execute.ts')
    window.pendingPipeline = runPipeline().then(() => ({ ok: true }), (error) => ({ ok: false, message: error.message }))
  })
  await page.waitForFunction(() => window.testRt.editor.getNode('generate').busy)
  await evaluate(async (id) => { await window.testStore.openGraph({ backendId: 'local', id }) }, ids.idB)
  await gateOff('__generateGate')
  const outcome = await evaluate(() => window.pendingPipeline)
  assert.equal(outcome.ok, false)
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').resultUrls.length), 0)
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').busy), false)
  console.log('PASS late generation cannot update a newly loaded node with the same ID')

  await setFlag('__generateMode', 'error')
  await page.locator('.run-btn').click()
  await page.waitForSelector('[data-node-id="generate"] .ui-error')
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').error), 'Mock upstream failure')
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').busy), false)
  console.log('PASS upstream failure updates node error and ends busy')

  // 刷新后：图文档（含参数与连线）与库收藏从 OPFS 恢复；运行状态不恢复。
  // 此时 localStorage 指向 B（晚返回场景切换后）：门控期间编辑的 prompt 已落盘。
  await page.reload()
  await page.waitForSelector('.ui-node[data-node-id="generate"]')
  await loadModules()
  const persisted = await evaluate(async () => {
    await new Promise((resolve) => setTimeout(resolve, 100))
    const tree = await (await import('/src/backends/registry.svelte.ts')).backendStore('local').storeTree()
    return {
      files: tree.files.length,
      title: document.querySelector('.graph-title .name')?.textContent,
      prompt: document.querySelector('textarea')?.value,
      results: document.querySelectorAll('.result-img').length,
      busy: window.testRt.editor.getNode('generate').busy,
    }
  })
  assert.equal(persisted.title, 'Graph B')
  assert.equal(persisted.prompt, 'Wait for save')
  assert.equal(persisted.files, 1)
  assert.equal(persisted.results, 0)
  assert.equal(persisted.busy, false)
  console.log('PASS reload restores graph/view/library from OPFS; runtime state does not persist')

  assert.deepEqual(pageErrors, [])
  console.log('PASS no uncaught browser errors')
} finally {
  await gateOff('__generateGate').catch(() => {})
  await gateOff('__saveGate').catch(() => {})
  await browser.close()
}
