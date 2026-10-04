import type { ImageRef } from './classes'
import type { DragImagePayload } from './dragPayload'
import type { GraphStoreFileMeta } from '../graphStore'
import { imageHash } from './imageHash.ts'

export type ImageSource = { kind: 'file'; file: File } | { kind: 'url'; image: DragImagePayload }
export interface ImportProgress { completed: number; total: number; name: string }
export interface ImportFailure { source: ImageSource; name: string; message: string }
export interface ImportReport { added: number; skipped: number; failures: ImportFailure[] }
interface ImportTarget {
	graphId: string
	isActive: () => boolean
	images: () => ImageRef[]
	append: (image: ImageRef) => boolean
}
interface ImportOptions {
	target: () => ImportTarget
	upload: (graphId: string, name: string, blob: Blob) => Promise<GraphStoreFileMeta>
	onProgress: (progress: ImportProgress | null) => void
	onReport: (report: ImportReport) => void
}

export function sourceName(source: ImageSource): string {
	if (source.kind === 'file') return source.file.name
	let raw = source.image.file
	if (!raw) {
		if (/^(data:|blob:)/i.test(source.image.url)) return 'image.png'
		try { raw = new URL(source.image.url, 'http://atelier.local').pathname }
		catch { return 'image.png' }
	}
	try { return decodeURIComponent(raw.split('/').pop() || 'image.png') }
	catch { return raw.split('/').pop() || 'image.png' }
}

/** 以实际文件签名确定格式，避免 URI 查询串 / 无扩展名 / 伪装文件影响入库。 */
async function prepareImage(blob: Blob, name: string): Promise<{ name: string; hash: string }> {
	const bytes = await blob.arrayBuffer()
	const head = new Uint8Array(bytes, 0, Math.min(bytes.byteLength, 12))
	const matches = (offset: number, signature: number[]) => signature.every((byte, i) => head[offset + i] === byte)
	const ext = matches(0, [137, 80, 78, 71, 13, 10, 26, 10]) ? 'png'
		: matches(0, [255, 216, 255]) ? 'jpg'
		: matches(0, [71, 73, 70, 56]) ? 'gif'
		: matches(0, [82, 73, 70, 70]) && matches(8, [87, 69, 66, 80]) ? 'webp' : ''
	if (!ext) throw new Error('请使用 PNG、JPEG、WebP 或 GIF 图片')
	const stem = name.replace(/\.[^.]+$/, '').replace(/[/\\:*?"<>|]/g, '_').replace(/\.{2,}/g, '_').trim() || 'image'
	const hash = await imageHash(bytes)
	return { name: `${stem}.${ext}`, hash }
}

/**
 * 连续拖入加入同一队列：顺序稳定、进度不中断、失败只影响当前图片。
 * 每次入队绑定原图 / 原节点；切图或删除后不能把异步结果写进新的节点。
 */
export function createImageImporter(opts: ImportOptions) {
	const jobs: { source: ImageSource; target: ImportTarget }[] = []
	let running: Promise<void> | null = null
	let disposed = false
	let completed = 0
	let total = 0
	let report: ImportReport = { added: 0, skipped: 0, failures: [] }
	const abort = new AbortController()

	async function drain() {
		try {
			while (jobs.length && !disposed) {
				const { source, target } = jobs.shift()!
				if (!target.isActive()) continue
				const name = sourceName(source)
				opts.onProgress({ completed, total, name })
				try {
					let blob: Blob
					if (source.kind === 'file') blob = source.file
					else {
						const res = await fetch(source.image.url, { signal: abort.signal })
						if (!res.ok) throw new Error(`读取失败（HTTP ${res.status}）`)
						blob = await res.blob()
					}
					if (disposed || !target.isActive()) continue
					const prepared = await prepareImage(blob, name)
					if (disposed || !target.isActive()) continue
					if (target.images().some((image) => image.hash === prepared.hash)) {
						report.skipped++
						continue
					}
					const meta = await opts.upload(target.graphId, prepared.name, blob)
					if (disposed || !target.isActive()) continue
					if (target.append({ file: meta.name, name, w: meta.w, h: meta.h, hash: prepared.hash })) report.added++
					else report.skipped++
				} catch (error) {
					if (!disposed && target.isActive()) {
						report.failures.push({ source, name, message: error instanceof Error ? error.message : String(error) })
					}
				} finally {
					completed++
				}
			}
			if (!disposed) opts.onReport(report)
		} finally {
			if (!disposed) opts.onProgress(null)
		}
	}

	return {
		enqueue(sources: ImageSource[]): Promise<void> {
			if (disposed || !sources.length) return running ?? Promise.resolve()
			const target = opts.target()
			if (!target.graphId || !target.isActive()) return Promise.resolve()
			if (!running) {
				completed = 0
				total = 0
				report = { added: 0, skipped: 0, failures: [] }
			}
			jobs.push(...sources.map((source) => ({ source, target })))
			total += sources.length
			if (!running) running = drain().finally(() => { running = null })
			else opts.onProgress({ completed, total, name: sourceName(jobs[0].source) })
			return running
		},
		dispose() {
			disposed = true
			jobs.length = 0
			abort.abort()
		},
	}
}
