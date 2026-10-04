// Runs in a real browser. All provider requests are direct fetches, without a proxy.
export async function probeRequest({ name, baseURL, token = '', kind, model, timeout = 25000 }) {
  const endpoint = kind === 'models' ? '/models' : kind === 'edit' ? '/images/edits' : '/images/generations';
  const headers = token ? { Authorization: `Bearer ${token}` } : {};
  const init = { method: kind === 'models' ? 'GET' : 'POST', headers, mode: 'cors', credentials: 'omit', signal: AbortSignal.timeout(timeout) };
  if (kind === 'edit') {
    const form = new FormData();
    form.set('model', '__cors_spike_invalid_model__');
    // Required prompt/image intentionally absent: tests multipart CORS without generating.
    init.body = form;
  } else if (kind !== 'models') {
    headers['Content-Type'] = 'application/json';
    init.body = JSON.stringify(kind === 'generate'
      ? { model, prompt: 'A small gray circle on a plain white background.', n: 1, quality: 'low', size: '1024x1024' }
      : {}); // Required model/prompt absent: validation-only request.
  }
  const started = performance.now();
  const clean = (message) => token ? String(message).split(token).join('[redacted]') : String(message);
  const result = { name, kind, origin: location.origin, authenticated: Boolean(token), endpoint: baseURL.replace(/\/$/, '') + endpoint };
  try {
    const response = await fetch(result.endpoint, init);
    const text = await response.text();
    result.readable = true;
    result.status = response.status;
    result.responseType = response.type;
    result.contentType = response.headers.get('content-type');
    let json;
    try { json = JSON.parse(text); } catch { /* Response readability still proves CORS. */ }
    result.json = Boolean(json);
    if (json?.error) result.apiError = clean(json.error.message ?? json.error).slice(0, 240);
    if (kind === 'generate' && response.ok) {
      result.imageCount = json?.data?.length ?? 0;
      const first = json?.data?.[0];
      let blob;
      if (first?.b64_json) {
        result.imageSource = 'base64';
        const bytes = Uint8Array.from(atob(first.b64_json), (c) => c.charCodeAt(0));
        blob = new Blob([bytes]);
      } else if (first?.url) {
        result.imageSource = 'url';
        result.imageHost = new URL(first.url).host; // Do not archive signed URL credentials.
        const imageResponse = await fetch(first.url, { mode: 'cors', credentials: 'omit', signal: AbortSignal.timeout(25000) });
        result.imageStatus = imageResponse.status;
        if (!imageResponse.ok) throw new Error(`Result image HTTP ${imageResponse.status}`);
        blob = await imageResponse.blob();
      }
      if (blob) {
        result.imageBytes = blob.size;
        const bitmap = await createImageBitmap(blob);
        result.imageSize = { width: bitmap.width, height: bitmap.height };
        bitmap.close();
        const root = await navigator.storage.getDirectory();
        const handle = await root.getFileHandle('cors-spike-result', { create: true });
        const writer = await handle.createWritable();
        await writer.write(blob);
        await writer.close();
        const saved = await handle.getFile();
        result.opfsReadback = saved.size === blob.size;
        await root.removeEntry('cors-spike-result');
      }
    }
  } catch (error) {
    result.readable ??= false;
    result.failure = { name: error.name, message: clean(error.message).slice(0, 240) };
  }
  result.durationMs = Math.round(performance.now() - started);
  return result;
}
