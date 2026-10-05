// Browser smoke: remote workspace mode. localStorage selects a mock server URL;
// page.route intercepts the absolute /api/* and /store/* requests, so no real
// server is needed. Verifies httpStore + http generator wiring end to end.
import assert from 'node:assert/strict'
import { pathToFileURL } from 'node:url'
import { resolve } from 'node:path'
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright')

const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a0S8AAAAASUVORK5CYII='
const pngBytes = Buffer.from(png.split(',')[1], 'base64')
const SERVER = 'http://127.0.0.1:5990'
const graph = {
  id: 'remote-graph', title: '远端图', group_id: null, version: 1,
  nodes: [
    { id: 'model', type: 'model', params: { provider: 'mock', modelId: 'gpt-image-2' } },
    { id: 'prompt', type: 'prompt', params: { text: 'Remote prompt' } },
    { id: 'generate', type: 'generate', params: {} },
    { id: 'preview', type: 'preview', params: {} },
  ],
  edges: [
    { id: 'm-g', source: 'model', sourcePort: 'model', target: 'generate', targetPort: 'model' },
    { id: 'p-g', source: 'prompt', sourcePort: 'text', target: 'generate', targetPort: 'prompt' },
    { id: 'g-p', source: 'generate', sourcePort: 'image', target: 'preview', targetPort: 'image' },
  ],
}

const pageErrors = []
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? 'chromium', headless: true })
const page = await browser.newPage({ viewport: { width: 1600, height: 1100 } })
page.on('pageerror', (error) => pageErrors.push(error.message))

let saveCount = 0
await page.route('**/api/**', async (route) => {
  const request = route.request(), url = new URL(request.url()), path = url.pathname, method = request.method()
  const json = (status, body) => route.fulfill({ status, contentType: 'application/json', body: JSON.stringify(body) })
  if (path === '/api/config') return json(200, { providers: [{ id: 'mock', name: 'Mock', models: ['gpt-image-2'] }], active_provider: 'mock' })
  if (path === '/api/groups') return json(200, [])
  if (path === '/api/graphs' && method === 'GET') return json(200, [graph])
  if (path === '/api/graphs/remote-graph' && method === 'GET') return json(200, graph)
  if (path === '/api/graphs/remote-graph' && method === 'PUT') { saveCount++; return json(200, {}) }
  if (path === '/api/graphs/remote-graph/view' && method === 'PUT') { saveCount++; return json(200, {}) }
  if (path === '/api/graphs/remote-graph/view' && method === 'GET') return json(200, { positions: {} })
  if (path === '/api/stores') return json(200, { dirs: [], files: [{ path: '远端库图.png', w: 1, h: 1, bytes: pngBytes.length }] })
  if (path === '/api/generate') return json(200, { imageUrls: [png, png] })
  throw new Error(`Unexpected remote API: ${method} ${path}`)
})
await page.route('**/store/**', (route) => route.fulfill({ status: 200, contentType: 'image/png', body: pngBytes }))

try {
  await page.addInitScript((server) => {
    localStorage.setItem('atelier-workspace', JSON.stringify({ kind: 'http', baseUrl: server }))
    localStorage.setItem('atelier-graph-id', 'remote-graph')
  }, SERVER)

  await page.goto(process.env.ATELIER_TEST_URL ?? 'http://127.0.0.1:5173')
  await page.waitForSelector('.ui-node[data-node-id="generate"]')
  await page.waitForFunction(() => !document.querySelector('.run-btn').disabled)

  // 顶栏指示远端；侧栏与库都来自 mock 服务
  assert.equal(await page.locator('.ws-entry span').first().textContent(), '远端')
  assert.equal(await page.locator('.graph-title .name').textContent(), '远端图')
  assert.equal(await page.locator('.entry[data-entry]').count(), 1)
  const thumb = await page.locator('.entry[data-entry] img').getAttribute('src')
  assert.ok(thumb.startsWith(`${SERVER}/store/`), `library thumb must be an absolute server URL, got ${thumb}`)

  // 生图走 /api/generate 代理；编辑后经 httpStore PUT 回传
  await page.locator('.run-btn').click()
  await page.waitForSelector('.result-img')
  assert.equal(await page.locator('.result-img').count(), 2)
  await page.evaluate(async () => {
    const { rt } = await import('/src/canvas/runtime.ts')
    const session = await import('/src/canvas/session.svelte.ts')
    rt.editor.getNode('prompt').text = 'Edited remotely'
    session.scheduleSave()
    await session.flushNow()
  })
  assert.ok(saveCount >= 1, 'edited graph must be PUT back to the server')

  // 工作区切换入口：popover 展示当前远端地址，选择仍持久化
  await page.locator('.ws-entry').click()
  await page.waitForSelector('.ws-pop')
  assert.ok(await page.locator('.ws-pop .ws-current').textContent().then((t) => t.includes(SERVER)))
  const stored = await page.evaluate(() => localStorage.getItem('atelier-workspace'))
  assert.equal(JSON.parse(stored).baseUrl, SERVER)

  assert.deepEqual(pageErrors, [])
  console.log('PASS remote workspace: httpStore serves graphs/library, absolute image URLs, proxied generation')
} finally {
  await browser.close()
}
