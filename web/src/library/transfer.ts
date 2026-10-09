import { backendStore } from '../backends/registry.svelte'
import { baseName, joinPath } from './paths'

/** Explicit copy keeps source ownership and captures both destinations before I/O. */
export async function copyStorePaths(sourceId: string, paths: string[], targetId: string, dir: string): Promise<void> {
	const source = backendStore(sourceId), target = backendStore(targetId)
	const tree = await source.storeTree()
	const selected = paths.filter((path) => !paths.some((other) => other !== path && path.startsWith(`${other}/`)))
	for (const path of selected) {
		const files = tree.files.filter((file) => file.path === path || file.path.startsWith(`${path}/`))
		if (tree.dirs.includes(path)) {
			await target.makeStoreDir(joinPath(dir, baseName(path)))
			for (const child of tree.dirs.filter((child) => child.startsWith(`${path}/`))) await target.makeStoreDir(joinPath(dir, `${baseName(path)}/${child.slice(path.length + 1)}`))
		}
		for (const file of files) {
			const url = await source.storeUrl(file.path)
			if (!url) throw new Error(`图片不存在：${file.path}`)
			const res = await fetch(url)
			if (!res.ok) throw new Error(`读取图片失败（HTTP ${res.status}）`)
			const blob = await res.blob()
			const relative = file.path === path ? baseName(path) : `${baseName(path)}/${file.path.slice(path.length + 1)}`
			const slash = relative.lastIndexOf('/')
			await target.uploadStoreFile(new File([blob], baseName(relative), { type: blob.type }), slash < 0 ? dir : joinPath(dir, relative.slice(0, slash)))
		}
	}
}
