/** 画布拖拽载荷：从底部库面板拖出的图片 */
export interface DragImagePayload {
	kind: 'store'
	url: string
	store: string
	file: string
	w?: number
	h?: number
}

export const IMAGE_DRAG_MIME = 'application/x-atelier-images'
export const STORE_DRAG_MIME = 'application/x-atelier-store'
export const IMAGE_ORDER_MIME = 'application/x-atelier-image-order'

/** 图片列表保留元信息；普通 URI 作为浏览器 / 外部应用的回退。 */
export function writeImageDrag(transfer: DataTransfer, images: DragImagePayload[]): void {
	transfer.setData(IMAGE_DRAG_MIME, JSON.stringify(images))
	transfer.setData('text/uri-list', images.map((image) => image.url).join('\r\n'))
	if (images[0]) transfer.setData('text/plain', images[0].url)
}

function imageUrl(value: unknown): value is string {
	return typeof value === 'string' && /^(https?:\/\/|blob:|data:image\/|\/(?!\/))/i.test(value)
}

/** drop 时读取（dragover 阶段浏览器禁止读数据），过滤注释、重复 URL 与坏载荷。 */
export function readImageDrag(transfer: Pick<DataTransfer, 'getData'>): DragImagePayload[] {
	let images: DragImagePayload[] = []
	try {
		const payload: unknown = JSON.parse(transfer.getData(IMAGE_DRAG_MIME) || 'null')
		if (Array.isArray(payload)) {
			images = payload.filter((item): item is DragImagePayload =>
				item?.kind === 'store' && imageUrl(item.url) && typeof item.file === 'string',
			)
		}
	} catch { /* 旧版本或外部应用仍可以走 URI 列表 */ }
	if (!images.length) {
		const raw = transfer.getData('text/uri-list') || transfer.getData('text/plain')
		images = raw.split(/\r?\n/).map((url) => url.trim())
			.filter(imageUrl)
			.map((url) => ({ kind: 'store', url, store: '', file: '' }))
	}
	const seen = new Set<string>()
	return images.filter(({ url }) => {
		if (seen.has(url)) return false
		seen.add(url)
		return true
	})
}
