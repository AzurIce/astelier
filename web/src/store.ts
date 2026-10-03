// 全局库（data/stores/）API 封装。平铺、无子目录。
// 「库」是用户显式收藏层：删图不动它，由用户显式删。
export interface StoreFileMeta {
	name: string
	w?: number
	h?: number
	bytes?: number
}

/** 全局库文件列表 */
export async function fetchStore(): Promise<StoreFileMeta[]> {
	const res = await fetch('/api/stores')
	if (!res.ok) throw new Error(`读取库失败（${res.status}）`)
	return res.json()
}

export async function uploadStoreFile(file: File): Promise<StoreFileMeta> {
	const res = await fetch(`/api/stores?filename=${encodeURIComponent(file.name)}`, {
		method: 'POST',
		body: file,
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return body as StoreFileMeta
}

export async function deleteStoreFile(name: string): Promise<void> {
	const res = await fetch(`/api/stores/${encodeURIComponent(name)}`, { method: 'DELETE' })
	if (!res.ok) throw new Error(`删除失败（${res.status}）`)
}

/** 全局库静态 URL */
export function storeUrl(name: string): string {
	return `/store/${encodeURIComponent(name)}`
}
