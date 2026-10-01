// image store（图内资产库）API 封装。
// 目录名 = 显示名（非 ASCII 原样保留，浏览器端 fetch 自动 percent-encode）。
// store 引用形如 { store: '参考图', file: '猫.png' }，静态寻址
// /gstore/{gid}/{store}/{file}。

export interface StoreFileMeta {
	name: string
	w?: number
	h?: number
	bytes?: number
}

export interface StoreInfo {
	/** 目录名（= 显示名，URL 段） */
	name: string
	title?: string
	files: StoreFileMeta[]
}

function gq(id: string, tail = ''): string {
	return `/api/graphs/${encodeURIComponent(id)}/stores${tail}`
}

export async function fetchStores(gid: string): Promise<StoreInfo[]> {
	const res = await fetch(gq(gid))
	if (!res.ok) throw new Error(`读取 image store 失败（${res.status}）`)
	return res.json()
}

export async function createStore(gid: string, name: string): Promise<StoreInfo> {
	const res = await fetch(gq(gid), {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name }),
	})
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `创建 store 失败（${res.status}）`)
	return { name: body.name as string, title: body.title as string, files: [] }
}

export async function renameStore(gid: string, name: string, next: string): Promise<void> {
	const res = await fetch(gq(gid, `/${encodeURIComponent(name)}`), {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ name: next }),
	})
	if (!res.ok) {
		const body = await res.json().catch(() => null)
		throw new Error(body?.error ?? `重命名失败（${res.status}）`)
	}
}

export async function deleteStore(gid: string, name: string): Promise<void> {
	const res = await fetch(gq(gid, `/${encodeURIComponent(name)}`), { method: 'DELETE' })
	if (!res.ok) throw new Error(`删除 store 失败（${res.status}）`)
}

export async function uploadStoreFile(gid: string, store: string, file: File): Promise<StoreFileMeta> {
	const res = await fetch(
		`${gq(gid, `/${encodeURIComponent(store)}/assets`)}?filename=${encodeURIComponent(file.name)}`,
		{ method: 'POST', body: file },
	)
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return body as StoreFileMeta
}

export async function deleteStoreFile(gid: string, store: string, name: string): Promise<void> {
	const res = await fetch(gq(gid, `/${encodeURIComponent(store)}/assets/${encodeURIComponent(name)}`), {
		method: 'DELETE',
	})
	if (!res.ok) throw new Error(`删除失败（${res.status}）`)
}

/** store 内图片的静态 URL（浏览器 fetch 时 percent-encode 路径段） */
export function storeFileUrl(gid: string, store: string, file: string): string {
	return `/gstore/${encodeURIComponent(gid)}/${encodeURIComponent(store)}/${encodeURIComponent(file)}`
}
