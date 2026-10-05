// Browser regression: real Svelte/Rete rendering, isolated mock application API.
// Start Vite first. All /api requests are intercepted; no provider/key is used.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright');
const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a0S8AAAAASUVORK5CYII=';
const fixture = (id) => ({ id, title: `Graph ${id}`, group_id: null, version: 1, nodes: [
  { id: 'model', type: 'model', params: {provider:'mock', modelId:'gpt-image-2'} },
  { id: 'prompt', type: 'prompt', params: {text:'Original prompt'} },
  { id: 'generate', type: 'generate', params: {n:2, quality:'low'} },
  { id: 'preview', type: 'preview', params: {} },
], edges: [
  {id:'m-g',source:'model',sourcePort:'model',target:'generate',targetPort:'model'},
  {id:'p-g',source:'prompt',sourcePort:'text',target:'generate',targetPort:'prompt'},
  {id:'g-p',source:'generate',sourcePort:'image',target:'preview',targetPort:'image'},
] });
const graphs = new Map(['A','B'].map((id) => [id, fixture(id)]));
const view = {version:1,positions:{model:{x:30,y:10},prompt:{x:30,y:190},generate:{x:420,y:10},preview:{x:790,y:10}}, outputs:{generate:png,preview:png}};
const requests = [], files = [], pageErrors = [];
let generateGate, saveGate, failSave = false, generateMode = 'success';
function deferred() { let release; const promise = new Promise((resolve) => {release = resolve}); return {promise,release}; }
const browser = await chromium.launch({executablePath:process.env.CHROMIUM_PATH ?? 'chromium',headless:true});
const page = await browser.newPage({viewport:{width:1600,height:1100}});
page.on('pageerror', (error) => pageErrors.push(error.message));
await page.route('**/api/**', async (route) => {
  const request = route.request(), url = new URL(request.url()), method = request.method();
  const path = url.pathname.slice(4);
  const record = {method,path,body:method === 'PUT' || path === '/generate' ? request.postDataJSON() : undefined};
  requests.push(record);
  const json = (status, body) => route.fulfill({status,contentType:'application/json',body:JSON.stringify(body)});
  if(path === '/config') return json(200,{providers:[{id:'mock',name:'Mock',models:['gpt-image-2']}],active_provider:'mock'});
  if(path === '/groups') return json(200,[]);
  if(path === '/graphs' && method === 'GET') return json(200,[...graphs.values()]);
  if(path === '/graphs' && method === 'POST') return json(200,graphs.get('A'));
  if(path === '/stores' && method === 'GET') return json(200,{dirs:[],files});
  if(path === '/stores' && method === 'POST') {
    const name = url.searchParams.get('filename');
    assert(name && name.length < 100, 'temporary URL must not be used as filename');
    files.push({path:name,bytes:request.postDataBuffer().length});
    return json(200,files.at(-1));
  }
  if(path === '/generate') {
    if(generateGate) await generateGate.promise;
    return generateMode === 'error' ? json(502,{error:'Mock upstream failure'}) : json(200,{imageUrls:[png,png]});
  }
  const match = path.match(/^\/graphs\/([^/]+)(\/view)?$/);
  if(match) {
    const [,id,suffix] = match;
    if(method === 'GET') return json(200,suffix ? view : graphs.get(id));
    if(method === 'PUT') {
      if(saveGate) await saveGate.promise;
      if(failSave) return json(500,{error:'Mock disk failure'});
      if(!suffix) graphs.set(id,{...graphs.get(id),...record.body});
      return json(200,{});
    }
  }
  throw new Error(`Unexpected application API: ${method} ${path}`);
});
await page.route('**/store/**', (route) => route.fulfill({status:200,contentType:'image/png',body:Buffer.from(png.split(',')[1],'base64')}));
const evaluate = (fn,args) => page.evaluate(fn,args);
const loadModules = async () => evaluate(async () => {
  const {rt} = await import('/src/canvas/runtime.ts');
  const store = await import('/src/canvas/session.svelte.ts');
  await store.flushNow();
  window.testRt = rt; window.testStore = store;
  window.nodeUpdates = 0;
  const originalUpdate = rt.area.update.bind(rt.area);
  rt.area.update = (...args) => {if(args[0] === 'node') window.nodeUpdates++; return originalUpdate(...args)};
});
try {
  await page.goto(process.env.ATELIER_TEST_URL ?? 'http://127.0.0.1:5173');
  await page.waitForSelector('.ui-node[data-node-id="generate"]');
  await page.waitForFunction(() => !document.querySelector('.run-btn').disabled);
  await loadModules();
  assert.equal(await page.locator('.result-img').count(),0,'legacy View outputs must not restore results');
  await evaluate(() => {window.testRt.editor.getNode('prompt').text='Reactive prompt';window.testRt.editor.getNode('generate').params.n=3});
  await page.waitForFunction(() => document.querySelector('textarea').value === 'Reactive prompt');
  assert.equal(await evaluate(() => window.nodeUpdates),0);
  console.log('PASS reactive node fields update real UI without area.update; legacy outputs ignored');

  generateGate = deferred();
  await page.locator('.run-btn').click();
  await page.waitForFunction(() => document.querySelector('[data-node-id="generate"]').classList.contains('busy'));
  const captured = requests.find((request) => request.path === '/generate').body;
  assert.equal(captured.prompt,'Reactive prompt'); assert.equal(captured.params.n,3);
  await evaluate(() => {window.testRt.editor.getNode('generate').params.n=9});
  assert.equal(captured.params.n,3);
  generateGate.release(); generateGate = null;
  await page.waitForSelector('.result-img');
  assert.equal(await page.locator('.result-img').count(),2);
  assert.equal(await evaluate(() => window.nodeUpdates),0);
  assert.equal(files.length,0);
  console.log('PASS busy and outputs react automatically; request snapshot stable; no automatic collection');
  await page.waitForFunction(() => {
    const path = document.querySelector('.ui-conn.t-image .wire');
    const socket = document.querySelector('[data-node-id="generate"] .port-row.output .ui-socket');
    if(!path || !socket) return false;
    const numbers = path.getAttribute('d').match(/-?\d+(?:\.\d+)?/g).map(Number);
    const point = new DOMPoint(numbers[0],numbers[1]).matrixTransform(path.getScreenCTM());
    const rect = socket.getBoundingClientRect();
    return Math.hypot(point.x-rect.left-rect.width/2,point.y-rect.top-rect.height/2)<2;
  });
  console.log('PASS connection endpoint follows reactive gallery height changes');

  await evaluate(async () => {
    const {LoadImageNode} = await import('/src/canvas/nodes/model.svelte.ts');
    const node = new LoadImageNode(); node.id = 'image';
    await window.testRt.editor.addNode(node);
    await window.testRt.area.translate(node.id,{x:30,y:450});
  });
  await page.waitForSelector('[data-node-id="image"] .image-editor');
  await evaluate((url) => {
    const transfer = new DataTransfer(); transfer.setData('text/uri-list',url);
    document.querySelector('[data-node-id="image"] .image-editor').dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:transfer}));
  },png);
  await page.waitForFunction(() => window.testRt.editor.getNode('image').images.length === 1);
  assert.equal(requests.filter((request) => request.method === 'POST' && /\/store$/.test(request.path)).length,0);
  const serialized = await evaluate(async () => {
    const {toDoc,toViewDoc} = await import('/src/canvas/document.ts');
    await window.testStore.flushNow();
    return {doc:toDoc(),view:toViewDoc()};
  });
  assert.deepEqual(serialized.doc.nodes.find((node) => node.id === 'image').params.images,[]);
  assert(!('outputs' in serialized.view));
  assert(!JSON.stringify(serialized).includes('data:image'));
  console.log('PASS temporary image drag creates session reference; Graph/View exclude runtime data');

  await evaluate((url) => {
    const transfer = new DataTransfer(); transfer.setData('text/uri-list',url);
    document.querySelector('.grid-wrap').dispatchEvent(new DragEvent('drop',{bubbles:true,cancelable:true,dataTransfer:transfer}));
  },png);
  await page.waitForSelector('.entry[data-entry]');
  assert.equal(files.length,1);
  console.log('PASS explicit drag into library persists one selected image');

  await evaluate(async () => {await window.testStore.openGraph('B')});
  assert.equal(await page.locator('.graph-title .name').textContent(),'Graph B');
  assert.equal(await page.locator('.result-img').count(),0);
  saveGate = deferred();
  await evaluate(() => {window.testRt.editor.getNode('prompt').text='Wait for save';window.testStore.scheduleSave()});
  await page.waitForFunction(() => window.testStore.graphSession.saveState === 'saving');
  await evaluate(() => {window.switchDone=false;window.switchPromise=window.testStore.openGraph('A').then(() => {window.switchDone=true})});
  assert.equal(await evaluate(() => window.testStore.activeGraphId()),'B');
  assert.equal(await evaluate(() => window.switchDone),false);
  saveGate.release(); saveGate = null;
  await evaluate(() => window.switchPromise);
  assert.equal(await evaluate(() => window.testStore.activeGraphId()),'A');
  assert.equal(await page.locator('.graph-title .name').textContent(),'Graph A');
  console.log('PASS graph switching awaits already-started writes and shares current title');

  failSave = true;
  await evaluate(() => {window.testStore.scheduleSave()});
  await page.waitForFunction(() => window.testStore.graphSession.saveState === 'error');
  const blocked = await evaluate(async () => {try {await window.testStore.openGraph('B');return false}catch{return true}});
  assert(blocked); assert.equal(await evaluate(() => window.testStore.activeGraphId()),'A');
  assert.equal(await page.locator('.save-dot').textContent().then((text) => text.trim()),'保存失败');
  failSave = false;
  await evaluate(() => window.testStore.flushNow());
  console.log('PASS failed save stays visible and blocks switching until retry succeeds');

  // Both fixtures deliberately reuse node IDs to exercise identity protection.
  generateGate = deferred();
  await evaluate(async () => {
    const {runPipeline} = await import('/src/canvas/execute.ts');
    window.pendingPipeline=runPipeline().then(() => ({ok:true}), (error) => ({ok:false,message:error.message}));
  });
  await page.waitForFunction(() => window.testRt.editor.getNode('generate').busy);
  await evaluate(() => window.testStore.openGraph('B'));
  generateGate.release(); generateGate = null;
  const outcome = await evaluate(() => window.pendingPipeline);
  assert.equal(outcome.ok,false);
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').resultUrls.length),0);
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').busy),false);
  console.log('PASS late generation cannot update a newly loaded node with the same ID');

  generateMode='error';
  await page.locator('.run-btn').click();
  await page.waitForSelector('[data-node-id="generate"] .ui-error');
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').error),'Mock upstream failure');
  assert.equal(await evaluate(() => window.testRt.editor.getNode('generate').busy),false);
  generateMode='success'; generateGate=deferred();
  await evaluate(async () => {
    const {runPipeline} = await import('/src/canvas/execute.ts');
    window.deletedNode=window.testRt.editor.getNode('generate');
    window.pendingPipeline=runPipeline().then(() => ({ok:true}), (error) => ({ok:false,message:error.message}));
  });
  await page.waitForFunction(() => window.deletedNode.busy);
  await evaluate(async () => {const {removeNodeCascade}=await import('/src/canvas/nodes/actions.ts');await removeNodeCascade('generate')});
  generateGate.release(); generateGate=null;
  assert.equal((await evaluate(() => window.pendingPipeline)).ok,false);
  assert.equal(await evaluate(() => window.deletedNode.busy),false);
  assert.equal(await evaluate(() => window.deletedNode.resultUrls.length),0);
  console.log('PASS deleting the executing node discards its late result and ends its busy state');
  assert.deepEqual(pageErrors,[]);
  console.log('PASS upstream failure updates node error and ends busy; no uncaught browser errors');
} finally {
  generateGate?.release(); saveGate?.release();
  await browser.close();
}
