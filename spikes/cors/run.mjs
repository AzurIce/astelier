import { createServer } from 'node:http';
import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { probeRequest } from './probe.mjs';

// Set PLAYWRIGHT_MODULE to an existing playwright(-core) entry point if it is
// installed outside this repository. No dependency or lockfile changes needed.
const { chromium } = await import(process.env.PLAYWRIGHT_MODULE
  ? pathToFileURL(resolve(process.env.PLAYWRIGHT_MODULE)).href : 'playwright');
const configPath = resolve(process.env.ATELIER_DATA_DIR ?? 'data', 'config.json');
const config = JSON.parse(await readFile(configPath, 'utf8'));
const providers = config.providers.map((provider) => {
  const stored = (provider.api_key ?? '').trim();
  const envName = stored.startsWith('env:') ? stored.slice(4).trim() : /^[A-Z_]+$/.test(stored) ? stored : null;
  return { id: provider.id, baseURL: provider.base_url.replace(/\/$/, ''), model: provider.models[0], token: envName ? process.env[envName] ?? '' : stored };
});
// OpenAI is an additional unauthenticated compatibility sample, not a configured provider.
if (!providers.some((provider) => provider.baseURL === 'https://api.openai.com/v1')) {
  providers.push({ id: 'openai-control', baseURL: 'https://api.openai.com/v1', token: '' });
}
const folder = fileURLToPath(new URL('.', import.meta.url));
const serve = async (path) => {
  const name = path === '/probe.mjs' ? 'probe.mjs' : 'index.html';
  return { contentType: name.endsWith('.mjs') ? 'text/javascript' : 'text/html', body: await readFile(resolve(folder, name)) };
};
const server = createServer(async (request, response) => {
  const { contentType, body } = await serve(new URL(request.url, 'http://localhost').pathname);
  response.writeHead(200, { 'Content-Type': contentType });
  response.end(body);
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
const localOrigin = `http://127.0.0.1:${server.address().port}`;
// Use a genuine HTTPS document. Playwright request interception can synthesize
// preflight responses, so do not use route()/fulfill() anywhere in this probe.
const httpsOrigin = 'https://example.com';
const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH ?? 'chromium', headless: true });
const report = { timestamp: new Date().toISOString(), browser: browser.version(), method: 'Real Chromium, default web security and TLS checks; direct provider requests; genuine localhost and example.com documents; no request interception', generate: process.argv.includes('--generate'), results: [] };
try {
  for (const origin of [localOrigin, httpsOrigin]) {
    const context = await browser.newContext();
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    await cdp.send('Network.enable');
    const network = [];
    const allowed = new Set(['access-control-allow-origin', 'access-control-allow-methods', 'access-control-allow-headers', 'access-control-allow-credentials', 'access-control-max-age']);
    cdp.on('Network.responseReceived', ({ type, response }) => {
      if (!providers.some((provider) => response.url.startsWith(provider.baseURL + '/'))) return;
      const headers = Object.fromEntries(Object.entries(response.headers).filter(([name]) => allowed.has(name.toLowerCase())));
      network.push({ type, status: response.status, url: response.url.split('?')[0], corsHeaders: headers });
    });
    await page.goto(origin);
    for (const provider of providers) {
      for (const kind of ['models', 'json', 'edit']) {
        const start = network.length;
        const result = await page.evaluate(probeRequest, { ...provider, name: `${provider.id}:${kind}`, kind });
        result.network = network.slice(start);
        report.results.push(result);
        console.log(JSON.stringify(result));
      }
    }
    // At most one real generation, from the HTTPS origin, only when explicitly requested.
    if (report.generate && origin === httpsOrigin) {
      const provider = providers.find((p) => p.token && p.id !== 'openai-control');
      const precheck = report.results.find((r) => r.origin === origin && r.name === `${provider?.id}:json`);
      if (provider && precheck?.readable) {
        console.log('Starting one generation: n=1, quality=low, size=1024x1024');
        const start = network.length;
        const result = await page.evaluate(probeRequest, { ...provider, name: `${provider.id}:generate`, kind: 'generate', timeout: 360000 });
        result.network = network.slice(start);
        report.results.push(result);
        console.log(JSON.stringify(result));
      } else console.log('Generation skipped: authenticated provider has no readable JSON response.');
    }
    await writeFile(resolve(folder, 'results.json'), JSON.stringify(report, null, 2) + '\n');
    await context.close();
  }
  await writeFile(resolve(folder, 'results.json'), JSON.stringify(report, null, 2) + '\n');
} finally {
  await browser.close();
  await new Promise((done) => server.close(done));
}
