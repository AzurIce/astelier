// Exercise menu pointer/click ordering and destructive actions through the actual UI.
import assert from 'node:assert/strict'
import { pathToFileURL } from 'node:url'
import { resolve } from 'node:path'
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright')
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? 'chromium', headless: true })
const page = await browser.newPage({ viewport: { width: 1280, height: 800 } })
const errors = []
page.on('pageerror', (error) => errors.push(error.message))
const tree = page.locator('.sidebar .tree')
const row = (name) => tree.getByRole('treeitem').filter({ has: page.locator('.name').getByText(name, { exact: true }) })
const menu = page.getByRole('menu')
const dialog = page.getByRole('alertdialog')
async function create(name, kind = 'graph') {
  await page.getByRole('button', { name: kind === 'graph' ? '新建图（根目录）' : '新建文件夹（根目录）', exact: true }).click()
  await tree.locator('input.rename').fill(name)
  await tree.locator('input.rename').press('Enter')
  await row(name).waitFor()
}
async function more(name) {
  await row(name).getByRole('button', { name: `更多操作：${name}`, exact: true }).click()
  await menu.waitFor()
}
try {
  await page.addInitScript(() => {
    if (!localStorage.getItem('sidebar-test-initialized')) {
      localStorage.setItem('astelier-backends', JSON.stringify([{ id: 'local', kind: 'opfs', name: '旧本地名称' }]))
      localStorage.setItem('sidebar-test-initialized', 'true')
    }
  })
  await page.goto(process.env.ASTELIER_TEST_URL ?? 'http://127.0.0.1:5173')
  await page.waitForFunction(() => document.querySelector('.run-btn') && !document.querySelector('.run-btn').disabled)
  assert.equal(await page.locator('.backend-root summary').innerText(), '浏览器存储')
  await page.getByRole('button', { name: '管理后端', exact: true }).click()
  assert.equal(await page.locator('[data-connection-id="local"] input').count(), 0)
  assert.equal(await page.locator('[data-connection-id="local"] .connection-name').innerText(), '浏览器存储')
  assert.equal(await page.evaluate(async () => {
    try { await (await import('/src/backends/registry.svelte.ts')).updateServer('local', '改名', 'http://example.com'); return false } catch { return true }
  }), true)
  await page.getByRole('button', { name: '关闭', exact: true }).click()

  const theme = page.getByRole('button', { name: '主题：默认', exact: true })
  await theme.locator('.dots').click()
  await page.getByRole('button', { name: /晨雾.*圆角柔和/ }).click()
  assert.equal(await page.locator('.skin-entry').innerText(), '主题：晨雾')
  assert.equal(await page.locator('.skin-pop').count(), 0)
  await page.keyboard.press('Escape')
  await page.reload()
  await page.getByRole('button', { name: '主题：晨雾', exact: true }).click()
  await page.getByRole('button', { name: /默认.*深色优先/ }).click()
  await page.keyboard.press('Escape')
  await page.waitForFunction(() => !document.querySelector('.run-btn').disabled)
  console.log('PASS explicit theme label, theme persistence and fixed browser storage name')

  const root = page.locator('.backend-root').first()
  const treeHeight = await tree.evaluate((el) => el.clientHeight)
  const rootHeight = await root.evaluate((el) => el.clientHeight)
  const rootWidth = await root.evaluate((el) => el.clientWidth)
  const roots = page.locator('.sidebar .roots')
  const rootsHeight = await roots.evaluate((el) => el.clientHeight)
  for (const [target, position] of [
    [tree, { x: 80, y: treeHeight - 12 }],
    [root.locator('summary'), { x: 70, y: 15 }],
    [root, { x: rootWidth - 1, y: rootHeight / 2 }],
    [roots, { x: 80, y: rootsHeight - 14 }],
  ]) {
    await target.click({ button: 'right', position })
    assert.equal(await menu.locator('.ui-menu-label').innerText(), '浏览器存储')
    assert.deepEqual(await menu.getByRole('menuitem').allTextContents(), ['新建图', '新建文件夹'])
    await page.keyboard.press('Escape')
  }
  await roots.click({ button: 'right', position: { x: 80, y: rootsHeight - 14 } })
  await menu.getByRole('menuitem', { name: '新建图', exact: true }).click()
  await tree.locator('input.rename').fill('空白处新建的图')
  await tree.locator('input.rename').press('Enter')
  await row('空白处新建的图').waitFor()
  console.log('PASS root menu from card whitespace, heading, border and sidebar background; create at root')

  await create('待删除图')
  await more('待删除图')
  await page.keyboard.press('Escape')
  assert.equal(await row('待删除图').getByRole('button').evaluate((el) => el === document.activeElement), true)
  await row('待删除图').click({ button: 'right' })
  await menu.getByRole('menuitem', { name: '删除', exact: true }).click()
  await dialog.getByRole('button', { name: '取消', exact: true }).click()
  assert.equal(await row('待删除图').count(), 1)
  await more('待删除图')
  await menu.getByRole('menuitem', { name: '重命名', exact: true }).click()
  await tree.locator('input.rename').fill('重命名后的图')
  await tree.locator('input.rename').press('Enter')
  await more('重命名后的图')
  await menu.getByRole('menuitem', { name: '删除', exact: true }).click()
  await dialog.getByRole('button', { name: '删除', exact: true }).click()
  await row('重命名后的图').waitFor({ state: 'detached' })
  await page.waitForFunction(() => !document.querySelector('.run-btn').disabled)
  assert.equal(await tree.getByRole('treeitem', { selected: true }).count(), 1)
  console.log('PASS right-click delete/cancel, more-menu rename/delete and active graph fallback')

  await create('待删除文件夹', 'dir')
  await row('待删除文件夹').locator('.twisty').click()
  assert.equal(await row('待删除文件夹').getAttribute('aria-expanded'), 'false')
  await more('待删除文件夹')
  await menu.getByRole('menuitem', { name: '新建文件夹', exact: true }).click()
  await tree.locator('input.rename').fill('子文件夹')
  await tree.locator('input.rename').press('Enter')
  await more('子文件夹')
  await menu.getByRole('menuitem', { name: '新建图', exact: true }).click()
  await tree.locator('input.rename').fill('保留的图')
  await tree.locator('input.rename').press('Enter')
  await row('保留的图').waitFor()
  const graphId = await row('保留的图').getAttribute('data-row-id')
  await more('待删除文件夹')
  await menu.getByRole('menuitem', { name: '删除', exact: true }).click()
  await dialog.getByRole('button', { name: '删除', exact: true }).click()
  await row('待删除文件夹').waitFor({ state: 'detached' })
  assert.equal(await row('子文件夹').count(), 0)
  await row('保留的图').waitFor()
  assert.equal(await page.evaluate(async (id) => (await (await import('/src/backends/registry.svelte.ts')).backendStore('local').fetchGraph(id)).group_id, graphId), null)

  await page.setViewportSize({ width: 800, height: 360 })
  await more('保留的图')
  const bounds = await menu.boundingBox()
  assert(bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= 800 && bounds.y + bounds.height <= 360)
  await page.keyboard.press('End')
  assert.equal(await menu.getByRole('menuitem', { name: '删除', exact: true }).evaluate((el) => el === document.activeElement), true)
  await page.keyboard.press('Escape')
  assert.deepEqual(errors, [])
  console.log('PASS folder deletion preserves nested graphs; folder toggle; menu fits viewport and supports keyboard')
} catch (error) {
  await page.screenshot({ path: '/tmp/astelier-sidebar-ui-failure.png' })
  console.error('Browser errors:', errors)
  throw error
} finally {
  await browser.close()
}
