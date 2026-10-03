/** 画布拖拽载荷：从底部库面板拖出的图片 */
export interface DragImagePayload {
	kind: 'store'
	url: string
	store: string
	file: string
	w?: number
	h?: number
}
