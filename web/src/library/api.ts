// 全局库 API 封装（委托当前工作区 store）。层级：任意深度子目录。
// 「库」是用户显式收藏层：删图不动它，由用户显式删；路径全部是相对路径。
import { workspaceStore } from '../workspace/store'
import type { StoreFileEntry, StoreTree } from '../workspace/types'

/** 全树：dirs 为全部目录（相对路径），files 为全部图片 */
export async function fetchStoreTree(): Promise<StoreTree> {
	return workspaceStore().storeTree()
}

/** 上传到指定子目录（dir 空 = 根） */
export async function uploadStoreFile(file: File, dir = ''): Promise<StoreFileEntry> {
	return workspaceStore().uploadStoreFile(file, dir)
}

/** 新建文件夹（可多级 "角色/猫"） */
export async function makeStoreDir(path: string): Promise<void> {
	await workspaceStore().makeStoreDir(path)
}

/** 重命名 / 移动（文件或目录整体） */
export async function moveStorePath(from: string, to: string): Promise<void> {
	await workspaceStore().moveStorePath(from, to)
}

/** 删除文件或目录（目录递归） */
export async function deleteStorePath(path: string): Promise<void> {
	await workspaceStore().deleteStorePath(path)
}

/** 展示 URL：本地为缓存 blob: URL，远端为绝对地址；文件缺失返回 null */
export async function storeUrl(path: string): Promise<string | null> {
	return workspaceStore().storeUrl(path)
}
