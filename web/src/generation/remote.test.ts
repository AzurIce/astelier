import { test } from 'node:test'
import assert from 'node:assert/strict'
import { createRemoteGenerator } from './remote'

test('remote execution binds the provider and transfers browser references as image bytes', async () => {
	const original = globalThis.fetch
	const calls: { url: string; init?: RequestInit }[] = []
	globalThis.fetch = (async (url, init) => {
		calls.push({ url: String(url), init })
		if (String(url).startsWith('blob:')) return new Response(new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]), { status: 200 })
		return Response.json({ imageUrls: ['data:image/png;base64,AA=='] })
	}) as typeof fetch
	try {
		await createRemoteGenerator('https://server.example', 'team/key').generate({ model: 'model:with:colon', prompt: 'cat', imageUrls: ['blob:private-browser-url'] })
		assert.equal(calls[1].url, 'https://server.example/api/providers/team%2Fkey/generate')
		const body = JSON.parse(calls[1].init!.body as string)
		assert.equal(body.model, 'model:with:colon')
		assert.ok(body.imageUrls[0].startsWith('data:image/png;base64,'))
		assert.ok(!JSON.stringify(body).includes('blob:'))
		assert.ok(!calls[1].init!.headers || !('Authorization' in calls[1].init!.headers))
	} finally { globalThis.fetch = original }
})
