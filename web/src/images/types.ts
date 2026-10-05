/** 节点引用的一张图：图内 store 文件名（内联感知：UI 只认「这张图」） */
export interface ImageRef {
	/** 未收藏产物的会话内图片，不写入图文档。 */
	dataUrl?: string
	/** 图内 store 文件名（即引用） */
	file: string
	/** 展示名（默认同 file） */
	name: string
	/** 展示用尺寸（来自已保存文件或导入信息） */
	w?: number
	h?: number
	/** 内容指纹：不同文件名的相同图片也只保留一份。 */
	hash?: string
}

export interface GraphStoreFileMeta {
	name: string
	w?: number
	h?: number
	bytes?: number
}

