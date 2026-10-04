/** Coalesces edits and serializes writes, including edits made during a write. */
export type SaveState = 'saved' | 'dirty' | 'saving' | 'error'
type SaveKind = 'graph' | 'view'

export class SaveQueue {
	private pending = new Set<SaveKind>()
	private timers = new Map<SaveKind, ReturnType<typeof setTimeout>>()
	private inFlight: Promise<void> | null = null

	private write: (kind: SaveKind) => Promise<void>
	private onState: (state: SaveState) => void

	constructor(write: (kind: SaveKind) => Promise<void>, onState: (state: SaveState) => void) {
		this.write = write
		this.onState = onState
	}

	schedule(kind: SaveKind, delay: number): void {
		this.pending.add(kind)
		this.onState(this.inFlight ? 'saving' : 'dirty')
		if (this.timers.has(kind)) return
		this.timers.set(kind, setTimeout(() => {
			this.timers.delete(kind)
			void this.drain().catch((error) => console.error('保存画布失败', error))
		}, delay))
	}

	/** Waits for pending edits AND any already-started writes. */
	async flush(): Promise<void> {
		for (const timer of this.timers.values()) clearTimeout(timer)
		this.timers.clear()
		while (this.inFlight || this.pending.size) await this.drain()
	}

	private drain(): Promise<void> {
		if (this.inFlight) return this.inFlight
		if (!this.pending.size) return Promise.resolve()
		this.onState('saving')
		this.inFlight = this.writePending().then(
			() => { this.onState(this.pending.size ? 'dirty' : 'saved') },
			(error) => { this.onState('error'); throw error },
		).finally(() => { this.inFlight = null })
		return this.inFlight
	}

	private async writePending(): Promise<void> {
		while (this.pending.size) {
			const kind = this.pending.values().next().value!
			this.pending.delete(kind)
			const timer = this.timers.get(kind)
			if (timer) clearTimeout(timer)
			this.timers.delete(kind)
			try {
				await this.write(kind)
			} catch (error) {
				// Keep dirty for a later edit or explicit retry; don't report success.
				this.pending.add(kind)
				for (const timer of this.timers.values()) clearTimeout(timer)
				this.timers.clear()
				throw error
			}
		}
	}
}
