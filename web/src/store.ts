// 全局库（data/stores/）API 封装。层级：任意深度子目录。
// 「库」是用户显式收藏层：删图不动它，由用户显式删；路径全部是相对路径。

export interface StoreFileEntry {
	/** 相对路径，如 "角色/猫.png" */
	path: string
	w?: number
	h?: number
	bytes: number
}

export interface StoreTree {
	dirs: string[]
	files: StoreFileEntry[]
}

async function api<T>(url: string, init?: RequestInit): Promise<T> {
	const res = await fetch(url, init)
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `请求失败（${res.status}）`)
	return body as T
}

/** 全树：dirs 为全部目录（相对路径），files 为全部图片 */
export async function fetchStoreTree(): Promise<StoreTree> {
	return api<StoreTree>('/api/stores')
}

/** 上传到指定子目录（dir 空 = 根） */
export async function uploadStoreFile(file: File, dir = ''): Promise<StoreFileEntry> {
	return api<StoreFileEntry>(
		`/api/stores?filename=${encodeURIComponent(file.name)}&dir=${encodeURIComponent(dir)}`,
		{ method: 'POST', body: file },
	)
}

/** 新建文件夹（可多级 "角色/猫"） */
export async function makeStoreDir(path: string): Promise<void> {
	await api('/api/stores/dirs', {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ path }),
	})
}

/** 重命名 / 移动（文件或目录整体） */
export async function moveStorePath(from: string, to: string): Promise<void> {
	await api(`/api/stores/${encodeStorePath(from)}`, {
		method: 'PATCH',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ to }),
	})
}

/** 删除文件或目录（目录递归） */
export async function deleteStorePath(path: string): Promise<void> {
	await fetch(`/api/stores/${encodeStorePath(path)}`, { method: 'DELETE' }).then((r) => {
		if (!r.ok) throw new Error(`删除失败（${r.status}）`)
	})
}

/** 静态 URL（层级路径按段编码，'/' 不编码） */
export function storeUrl(path: string): string {
	return `/store/${encodeStorePath(path)}`
}

function encodeStorePath(path: string): string {
	return path
		.split('/')
		.map((s) => encodeURIComponent(s))
		.join('/')
}

/** 路径工具：父目录 / 文件名 / 拼接 */
export function parentDir(path: string): string {
	const i = path.lastIndexOf('/')
	return i < 0 ? '' : path.slice(0, i)
}

export function baseName(path: string): string {
	const i = path.lastIndexOf('/')
	return i < 0 ? path : path.slice(i + 1)
}

export function joinPath(dir: string, name: string): string {
	return dir ? `${dir}/${name}` : name
}
