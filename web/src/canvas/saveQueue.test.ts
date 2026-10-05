import { test } from 'node:test'
import assert from 'node:assert/strict'
import { SaveQueue, type SaveState } from './saveQueue.ts'

function deferred() {
	let resolve!: () => void
	const promise = new Promise<void>((done) => { resolve = done })
	return { promise, resolve }
}

const turn = () => new Promise<void>((resolve) => setTimeout(resolve, 0))

test('flush waits for a write already started by the timer', async () => {
	const gate = deferred()
	let started = false
	const queue = new SaveQueue(async () => { started = true; await gate.promise }, () => {})
	queue.schedule('graph', 0)
	await turn()
	assert.equal(started, true)
	let finished = false
	const flushing = queue.flush().then(() => { finished = true })
	await turn()
	assert.equal(finished, false)
	gate.resolve()
	await flushing
	assert.equal(finished, true)
})

test('edits during a write are saved afterwards, with no concurrent writes', async () => {
	const gate = deferred()
	let value = 'before'
	let active = 0
	let maxActive = 0
	const snapshots: string[] = []
	const states: SaveState[] = []
	const queue = new SaveQueue(async () => {
		active++
		maxActive = Math.max(maxActive, active)
		snapshots.push(value)
		if (snapshots.length === 1) await gate.promise
		active--
	}, (state) => states.push(state))
	queue.schedule('graph', 10000)
	const flushing = queue.flush()
	value = 'after'
	queue.schedule('graph', 10000)
	gate.resolve()
	await flushing
	assert.deepEqual(snapshots, ['before', 'after'])
	assert.equal(maxActive, 1)
	assert.equal(states.at(-1), 'saved')
})

test('failed writes remain pending, reject flush, and can be retried', async () => {
	let fails = true
	const writes: string[] = []
	let state: SaveState = 'saved'
	const queue = new SaveQueue(async (kind) => {
		writes.push(kind)
		if (fails) throw new Error('disk unavailable')
	}, (next) => { state = next })
	queue.schedule('view', 10000)
	await assert.rejects(queue.flush(), /disk unavailable/)
	assert.equal(state, 'error')
	fails = false
	await queue.flush()
	assert.deepEqual(writes, ['view', 'view'])
	assert.equal(state, 'saved')
})

test('graph and view are both drained before flush resolves', async () => {
	const writes: string[] = []
	const queue = new SaveQueue(async (kind) => { writes.push(kind) }, () => {})
	queue.schedule('graph', 10000)
	queue.schedule('view', 10000)
	queue.schedule('graph', 10000)
	await queue.flush()
	assert.deepEqual(writes, ['graph', 'view'])
})

test('failure of one document retains both it and the other pending document', async () => {
	let fails = true
	const writes: string[] = []
	const queue = new SaveQueue(async (kind) => {
		writes.push(kind)
		if (fails) throw new Error('offline')
	}, () => {})
	queue.schedule('graph', 10000)
	queue.schedule('view', 10000)
	await assert.rejects(queue.flush(), /offline/)
	fails = false
	await queue.flush()
	assert.deepEqual(writes, ['graph', 'view', 'graph'])
})


test('flush includes an edit scheduled as the previous write finishes', async () => {
	let writes = 0
	const queue = new SaveQueue(async () => { writes++ }, (state) => {
		// A synchronous subscriber can schedule another edit before inFlight clears.
		if (state === 'saved' && writes === 1) queue.schedule('graph', 10000)
	})
	queue.schedule('graph', 10000)
	await queue.flush()
	assert.equal(writes, 2)
})
