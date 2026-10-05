// 图内持久参考图的上传、读取 URL 与回收。委托当前工作区 store；
// 展示 URL 在本地模式下是缓存的 blob: 对象 URL（异步解析）。
import type { GraphStoreFileMeta } from '../images/types'
import { workspaceStore } from './store'

/** 上传 / 复制进图内 store（文件名即引用；同名同内容幂等，异内容加序号） */
export async function uploadGraphStoreFile(
	gid: string,
	name: string,
	blob: Blob,
): Promise<GraphStoreFileMeta> {
	return workspaceStore().uploadGraphStoreFile(gid, name, blob)
}

/** 展示 / 执行用 URL：本地为缓存 blob: URL，远端为绝对地址；文件缺失返回 null */
export async function graphStoreUrl(gid: string, name: string): Promise<string | null> {
	return workspaceStore().graphStoreUrl(gid, name)
}

/** 回收图内已解除引用的文件；调用方决定回收失败如何处理。 */
export async function deleteGraphStoreFile(gid: string, name: string): Promise<void> {
	await workspaceStore().deleteGraphStoreFile(gid, name)
}
