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
