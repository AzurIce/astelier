// 图内持久参考图的上传、读取 URL 与回收。
import type { GraphStoreFileMeta } from '../images/types'

/** 上传 / 复制进图内 store（文件名即引用） */
export async function uploadGraphStoreFile(
	gid: string,
	name: string,
	blob: Blob,
): Promise<GraphStoreFileMeta> {
	const res = await fetch(
		`/api/graphs/${encodeURIComponent(gid)}/store?filename=${encodeURIComponent(name)}`,
		{ method: 'POST', body: blob },
	)
	const body = await res.json().catch(() => null)
	if (!res.ok) throw new Error(body?.error ?? `上传失败（${res.status}）`)
	return body as GraphStoreFileMeta
}

/** 图内 store 静态 URL */
export function graphStoreUrl(gid: string, name: string): string {
	return `/gstore/${encodeURIComponent(gid)}/${encodeURIComponent(name)}`
}

/** 回收图内已解除引用的文件；调用方决定回收失败如何处理。 */
export async function deleteGraphStoreFile(gid: string, name: string): Promise<void> {
	const res = await fetch(`/api/graphs/${encodeURIComponent(gid)}/store/${encodeURIComponent(name)}`, { method: 'DELETE' })
	if (!res.ok) throw new Error(`删除参考图失败（${res.status}）`)
}
