// 库与图内参考图的路径/文件名校验，以及图目录名净化。
// 移植自 Rust store.rs（safe_store_file / safe_store_path /
// resolve_store_dir_path / sanitize_dir_name / free_store_name），语义保持一致：
// 路径段经过校验后才拼接，OPFS 写入永远不接收未校验的用户输入。

const IMAGE_EXTS = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif'])
const BAD_SEG_CHARS = /[/\\:*?"<>|]/
/** 全局版仅用于 replace；.test() 用上面的无标志版，避免 lastIndex 状态 */
const STRIP_BAD_SEG_CHARS = /[/\\:*?"<>|]/g

/** 图片文件名：单段 + 扩展名白名单（jpeg 归一为 jpg），非法返回 null */
export function safeStoreFile(name: string): string | null {
	if (name.includes('/') || name.includes('\\') || name.includes('..')) return null
	const dot = name.lastIndexOf('.')
	if (dot < 0) return null
	const stem = name.slice(0, dot)
	let ext = name.slice(dot + 1).toLowerCase()
	if (!IMAGE_EXTS.has(ext)) return null
	if (ext === 'jpeg') ext = 'jpg'
	const cleanStem = stem.replace(STRIP_BAD_SEG_CHARS, '').trim()
	if (!cleanStem) return null
	return `${cleanStem}.${ext}`
}

/**
 * 库相对路径：按 '/' 分段（≤8 段、总长 ≤240），目录段容忍可见字符，
 * 末段按图片文件名规则校验。校验通过的段不含 `..` 与非法字符，
 * 因此拼出的路径必然落在库根内。
 */
export function safeStorePath(path: string): string | null {
	const trimmed = path.trim()
	if (!trimmed || trimmed.length > 240) return null
	const segs = trimmed.split('/')
	if (segs.length > 8) return null
	const out: string[] = []
	for (let i = 0; i < segs.length; i++) {
		const seg = segs[i].trim()
		if (!seg || seg === '.' || seg === '..' || BAD_SEG_CHARS.test(seg)) return null
		if (i + 1 === segs.length) {
			const name = safeStoreFile(seg)
			if (!name) return null
			out.push(name)
		} else {
			out.push(seg)
		}
	}
	return out.join('/')
}

/** 目录路径（末段也按目录名规则，无扩展名要求） */
export function resolveStoreDirPath(path: string): string | null {
	const trimmed = path.trim()
	if (!trimmed || trimmed.length > 240) return null
	const segs = trimmed.split('/')
	if (segs.length > 8) return null
	const out: string[] = []
	for (const raw of segs) {
		const seg = raw.trim()
		if (!seg || seg === '.' || seg === '..' || BAD_SEG_CHARS.test(seg)) return null
		out.push(seg)
	}
	return out.join('/')
}

/** 图目录名净化：非法字符替换为 -、折叠空白、限长 64 字符、去首尾点 */
export function sanitizeDirName(title: string): string {
	const cleaned = title.trim().replace(/[/\\:*?"<>|]/g, '-')
	const collapsed = cleaned.split(/\s+/).filter(Boolean).join(' ')
	const limited = Array.from(collapsed).slice(0, 64).join('').trim()
	return limited.replace(/^\.+|\.+$/g, '') || '未命名图'
}

/**
 * 取一个未被占用的文件名：`x.png` → `x-2.png`、`x-3.png`…
 * 占用判定由调用方给出（实现侧先列目录再同步判断）；
 * 2–999 全被占时用 fallbackId 兜底，与 Rust 侧一致。
 */
export function freeStoreName(name: string, taken: (candidate: string) => boolean, fallbackId: () => string): string {
	const dot = name.lastIndexOf('.')
	const stem = dot >= 0 ? name.slice(0, dot) : name
	const ext = dot >= 0 ? name.slice(dot) : ''
	for (let n = 2; n < 1000; n++) {
		const candidate = `${stem}-${n}${ext}`
		if (!taken(candidate)) return candidate
	}
	return `${stem}-${fallbackId()}${ext}`
}
