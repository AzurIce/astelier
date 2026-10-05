// OPFS 文件系统原语。工作区全部读写经过这里：
// 路径用「已校验的段数组」表达（['graphs', gid, 'graph.json']），
// 不接收拼接好的用户输入字符串；校验见 paths.ts。
//
// JSON 直接覆盖写（OPFS 无 rename 覆盖原子性），失败冒泡给调用方，
// 由保存队列维持「保存失败」语义。

export type FsPath = string[]

/** 工作区在 OPFS 根下的命名空间（同源多应用互不干扰） */
export const WORKSPACE_ROOT: FsPath = ['atelier']

export async function opfsRoot(): Promise<FileSystemDirectoryHandle> {
	return navigator.storage.getDirectory()
}

export function uuid(): string {
	return crypto.randomUUID().replaceAll('-', '')
}

async function dirHandle(path: FsPath, create: boolean): Promise<FileSystemDirectoryHandle> {
	let dir = await opfsRoot()
	for (const seg of path) dir = await dir.getDirectoryHandle(seg, { create })
	return dir
}

async function parentOf(path: FsPath): Promise<{ dir: FileSystemDirectoryHandle; name: string }> {
	if (path.length === 0) throw new Error('路径不能为空')
	const dir = await dirHandle(path.slice(0, -1), true)
	return { dir, name: path[path.length - 1] }
}

export async function readBytes(path: FsPath): Promise<Uint8Array<ArrayBuffer> | null> {
	try {
		const { dir, name } = await parentOf(path)
		const handle = await dir.getFileHandle(name)
		const file = await handle.getFile()
		return new Uint8Array(await file.arrayBuffer())
	} catch (e) {
		if ((e as DOMException)?.name === 'NotFoundError') return null
		throw new Error(`读取 ${path.join('/')} 失败：${describe(e)}`)
	}
}

export async function readFile(path: FsPath): Promise<File | null> {
	try {
		const { dir, name } = await parentOf(path)
		const handle = await dir.getFileHandle(name)
		return await handle.getFile()
	} catch (e) {
		if ((e as DOMException)?.name === 'NotFoundError') return null
		throw new Error(`读取 ${path.join('/')} 失败：${describe(e)}`)
	}
}

export async function readJson<T>(path: FsPath): Promise<T | null> {
	const bytes = await readBytes(path)
	if (!bytes) return null
	try {
		return JSON.parse(new TextDecoder().decode(bytes)) as T
	} catch {
		return null
	}
}

/**
 * 跨标签页写串行化（Web Locks，同源生效；不可用时退化为直接执行）。
 * 对应 Rust 侧全局 STORE_LOCK：读改写序列（groups/config/graph 合并）在
 * 持锁期间整体执行，避免两个标签页交错覆盖。
 */
export async function withFsLock<T>(fn: () => Promise<T>): Promise<T> {
	if (typeof navigator !== 'undefined' && 'locks' in navigator) {
		return navigator.locks.request('atelier-opfs-write', fn)
	}
	return fn()
}

export async function writeBytes(path: FsPath, bytes: BufferSource): Promise<void> {
	const { dir, name } = await parentOf(path)
	const handle = await dir.getFileHandle(name, { create: true })
	const stream = await handle.createWritable()
	try {
		await stream.write(bytes)
	} finally {
		await stream.close()
	}
}

/** pretty JSON，便于导出后直接查看 */
export async function writeJson(path: FsPath, value: unknown): Promise<void> {
	await writeBytes(path, new TextEncoder().encode(JSON.stringify(value, null, 2)))
}

/** 删除文件或目录（目录递归）；不存在不报错 */
export async function removePath(path: FsPath): Promise<void> {
	const { dir, name } = await parentOf(path)
	try {
		await dir.removeEntry(name, { recursive: true })
	} catch (e) {
		if ((e as DOMException)?.name === 'NotFoundError') return
		throw new Error(`删除 ${path.join('/')} 失败：${describe(e)}`)
	}
}

export async function fileExists(path: FsPath): Promise<boolean> {
	try {
		const { dir, name } = await parentOf(path)
		await dir.getFileHandle(name)
		return true
	} catch {
		return false
	}
}

export async function dirExists(path: FsPath): Promise<boolean> {
	try {
		await dirHandle(path, false)
		return true
	} catch {
		return false
	}
}

/** 确保目录存在（多级创建） */
export async function ensureDir(path: FsPath): Promise<void> {
	await dirHandle(path, true)
}

export interface DirEntries {
	dirs: Set<string>
	files: Set<string>
}

/** 列出某层目录名（不存在返回空集） */
export async function listDir(path: FsPath): Promise<DirEntries> {
	const entries: DirEntries = { dirs: new Set(), files: new Set() }
	let dir: FileSystemDirectoryHandle
	try {
		dir = await dirHandle(path, false)
	} catch {
		return entries
	}
	// TS dom lib 未给 FileSystemDirectoryHandle 声明 entries()，按规范异步迭代 [name, handle]
	for await (const [name, handle] of dir as unknown as AsyncIterable<[string, FileSystemHandle]>) {
		if (handle.kind === 'directory') entries.dirs.add(name)
		else entries.files.add(name)
	}
	return entries
}

/** 目录名去重：已存在则 -2 / -3…（建图时用，目录名只在建图时确定） */
export async function uniqueDirName(parent: FsPath, base: string): Promise<string> {
	const taken = await listDir(parent)
	for (let n = 1; ; n++) {
		const candidate = n === 1 ? base : `${base}-${n}`
		if (!taken.dirs.has(candidate) && !taken.files.has(candidate)) return candidate
	}
}

/**
 * 移动文件或目录（递归复制后删除原路径）。OPFS 的 handle.move 对目录
 * 支持有限，统一走复制删除；库的移动/改名频率低，代价可接受。
 */
export async function movePath(from: FsPath, to: FsPath): Promise<void> {
	if ((await readFile(from)) !== null) {
		const bytes = await readBytes(from)
		if (bytes) await writeBytes(to, bytes)
		await removePath(from)
		return
	}
	if (await dirExists(from)) {
		await copyDir(from, to)
		await removePath(from)
	}
}

async function copyDir(from: FsPath, to: FsPath): Promise<void> {
	const entries = await listDir(from)
	for (const name of entries.dirs) await copyDir([...from, name], [...to, name])
	for (const name of entries.files) {
		const bytes = await readBytes([...from, name])
		if (bytes) await writeBytes([...to, name], bytes)
	}
}

function describe(e: unknown): string {
	if (e instanceof Error) return e.message
	return String(e)
}
