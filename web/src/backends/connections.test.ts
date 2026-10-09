import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeServerUrl, parseConnections } from './connections'

test('connections retain stable IDs, one local root and distinct server roots', () => {
	const entries = parseConnections(JSON.stringify([
		{ id: 'local', kind: 'opfs', name: '我的本地' },
		{ id: 'a', kind: 'http', name: 'A', baseUrl: 'https://a.example/v1/' },
		{ id: 'b', kind: 'http', name: 'B', baseUrl: 'https://b.example' },
		{ id: 'duplicate', kind: 'http', baseUrl: 'https://a.example/v1' },
		{ id: 'local', kind: 'http', baseUrl: 'https://evil.example' },
		{ id: 'invalid', kind: 'http', baseUrl: 'javascript:alert(1)' },
	]))
	assert.deepEqual(entries.map((entry) => entry.id), ['local', 'a', 'b'])
	assert.equal(entries[0].name, '我的本地')
	assert.equal(entries[1].baseUrl, 'https://a.example/v1')
	assert.deepEqual(parseConnections('{broken').map((entry) => entry.id), ['local'])
})

test('server URLs reject credentials, query strings and unsupported protocols', () => {
	assert.equal(normalizeServerUrl(' https://server.example/astelier/ '), 'https://server.example/astelier')
	for (const url of ['ftp://server.example', 'https://user:secret@server.example', 'https://server.example?key=secret', 'https://server.example#fragment']) assert.throws(() => normalizeServerUrl(url))
})
