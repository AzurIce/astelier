import { rt } from './runtime'
import { scheduleViewSave } from './session.svelte'

/* ---------------- 视口控制（按钮 / 适配 / 缩放百分比） ---------------- */

const zoomSubs = new Set<(k: number) => void>()

export function subscribeZoom(cb: (k: number) => void): () => void {
	zoomSubs.add(cb)
	return () => zoomSubs.delete(cb)
}

export function notifyZoom(k: number) {
	for (const cb of zoomSubs) cb(k)
}

/** 画布容器（#rete） */
function canvasEl(): HTMLElement | null {
	return (rt.area?.area.content.holder.parentElement as HTMLElement | null) ?? null
}

/** 以指定 client 点（缺省画布中心）为锚缩放 */
export function zoomBy(factor: number, clientX?: number, clientY?: number): void {
	const area = rt.area
	if (!area) return
	const rect = canvasEl()?.getBoundingClientRect()
	if (!rect) return
	const t = area.area.transform
	const px = clientX != null ? clientX - rect.left : rect.width / 2
	const py = clientY != null ? clientY - rect.top : rect.height / 2
	const next = Math.min(2.5, Math.max(0.2, t.k * factor))
	if (next === t.k) return
	const k = t.k
	t.k = next
	t.x = px - ((px - t.x) * next) / k
	t.y = py - ((py - t.y) * next) / k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${next})`
	notifyZoom(next)
	scheduleViewSave()
}

/** 缩放到 100% */
export function resetZoom(): void {
	const area = rt.area
	if (!area) return
	const rect = canvasEl()?.getBoundingClientRect()
	if (!rect) return
	const t = area.area.transform
	const px = rect.width / 2
	const py = rect.height / 2
	const k = t.k
	const next = 1
	t.k = next
	t.x = px - ((px - t.x) * next) / k
	t.y = py - ((py - t.y) * next) / k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${next})`
	notifyZoom(next)
	scheduleViewSave()
}

/** 全部节点收入视野 */
export function fitView(): void {
	const area = rt.area
	if (!area) return
	const views = [...area.nodeViews.values()]
	if (views.length === 0) return
	let minX = Infinity
	let minY = Infinity
	let maxX = -Infinity
	let maxY = -Infinity
	for (const v of views) {
		const w = v.element.offsetWidth || 220
		const h = v.element.offsetHeight || 140
		minX = Math.min(minX, v.position.x)
		minY = Math.min(minY, v.position.y)
		maxX = Math.max(maxX, v.position.x + w)
		maxY = Math.max(maxY, v.position.y + h)
	}
	const el = canvasEl()
	const w = el?.clientWidth || 1200
	const h = el?.clientHeight || 800
	const pad = 96
	const k = Math.min(1.4, Math.max(0.2, Math.min((w - pad) / (maxX - minX), (h - pad) / (maxY - minY))))
	const t = area.area.transform
	t.k = k
	t.x = (w - (maxX - minX) * k) / 2 - minX * k
	t.y = (h - (maxY - minY) * k) / 2 - minY * k
	area.area.content.holder.style.transform = `translate(${t.x}px, ${t.y}px) scale(${k})`
	notifyZoom(k)
	scheduleViewSave()
}

export function currentZoom(): number {
	return rt.area?.area.transform.k ?? 1
}
