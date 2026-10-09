// 图内参考图与库文件的展示 URL：读 OPFS File 生成 blob: URL 并按引用缓存。
// Blob 由磁盘背书，常驻缓存的内存压力可控；文件删除/移动时释放。
// 远端工作区不经过这里（图片 URL 是服务端绝对地址）。
import { readFile, WORKSPACE_ROOT, type FsPath } from './fs'

const graphStoreUrls = new Map<string, string>()
const storeUrls = new Map<string, string>()

function graphStorePath(gid: string, name: string): FsPath {
	return [...WORKSPACE_ROOT, 'graphs', gid, 'store', name]
}

function storePath(path: string): FsPath {
	return [...WORKSPACE_ROOT, 'stores', ...path.split('/')]
}

export async function graphStoreObjectUrl(gid: string, name: string): Promise<string | null> {
	const key = `${gid}\u0000${name}`
	const cached = graphStoreUrls.get(key)
	if (cached) return cached
	const file = await readFile(graphStorePath(gid, name))
	if (!file) return null
	const url = URL.createObjectURL(file)
	graphStoreUrls.set(key, url)
	return url
}

export async function storeObjectUrl(path: string): Promise<string | null> {
	const cached = storeUrls.get(path)
	if (cached) return cached
	const file = await readFile(storePath(path))
	if (!file) return null
	const url = URL.createObjectURL(file)
	storeUrls.set(path, url)
	return url
}

/** 释放图内文件的对象 URL：name 省略时释放该图全部 */
export function releaseGraphStoreObjectUrls(gid: string, name?: string): void {
	for (const [key, url] of graphStoreUrls) {
		if (key.startsWith(`${gid}\u0000`) && (name === undefined || key === `${gid}\u0000${name}`)) {
			URL.revokeObjectURL(url)
			graphStoreUrls.delete(key)
		}
	}
}

export function releaseStoreObjectUrl(path: string): void {
	for (const [key, url] of storeUrls) {
		if (key === path || key.startsWith(`${path}/`)) { URL.revokeObjectURL(url); storeUrls.delete(key) }
	}
}
