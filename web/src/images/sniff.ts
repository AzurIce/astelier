// 图片字节嗅探：格式签名 → 扩展名 / MIME，PNG / JPEG / WebP 头解析宽高。
// 移植自 Rust store.rs sniff_dimensions 与 adapter.rs sniff_ext，语义保持一致；
// 库列表、上传元信息与生图结果解码共用这一份实现。

const starts = (bytes: Uint8Array, signature: number[], at = 0): boolean =>
	signature.every((byte, i) => bytes[at + i] === byte)

const PNG = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
const JPEG = [0xff, 0xd8, 0xff]
const GIF = [0x47, 0x49, 0x46, 0x38]
const RIFF = [0x52, 0x49, 0x46, 0x46]
const WEBP = [0x57, 0x45, 0x42, 0x50]

/** 按魔数识别扩展名；未知签名按 PNG 处理（与 Rust 侧行为一致） */
export function sniffExt(bytes: Uint8Array): 'png' | 'jpg' | 'webp' | 'gif' {
	if (starts(bytes, PNG)) return 'png'
	if (starts(bytes, JPEG)) return 'jpg'
	if (bytes.length > 12 && starts(bytes, RIFF) && starts(bytes, WEBP, 8)) return 'webp'
	if (starts(bytes, GIF)) return 'gif'
	return 'png'
}

export function imageMime(ext: string): string {
	switch (ext) {
		case 'jpg': case 'jpeg': return 'image/jpeg'
		case 'webp': return 'image/webp'
		case 'gif': return 'image/gif'
		default: return 'image/png'
	}
}

export interface Dimensions {
	w: number | null
	h: number | null
}

const be16 = (bytes: Uint8Array, at: number): number => (bytes[at] << 8) | bytes[at + 1]

/** 从 PNG / JPEG / WebP 字节里嗅探宽高（尽力而为，未知返回 null） */
export function sniffDimensions(bytes: Uint8Array): Dimensions {
	if (bytes.length > 24 && starts(bytes, PNG)) {
		return {
			w: ((bytes[16] << 24) | (bytes[17] << 16) | (bytes[18] << 8) | bytes[19]) >>> 0,
			h: ((bytes[20] << 24) | (bytes[21] << 16) | (bytes[22] << 8) | bytes[23]) >>> 0,
		}
	}
	if (bytes.length > 3 && starts(bytes, JPEG)) {
		// 逐段扫描 SOF（C0–CF，跳过 C4/C8/CC），容忍 EXIF 段抢占
		let i = 2
		while (i + 9 < bytes.length) {
			if (bytes[i] !== 0xff) break
			const marker = bytes[i + 1]
			const len = be16(bytes, i + 2)
			if (marker >= 0xc0 && marker <= 0xcf && marker !== 0xc4 && marker !== 0xc8 && marker !== 0xcc) {
				return { h: be16(bytes, i + 5), w: be16(bytes, i + 7) }
			}
			i += 2 + len
		}
	}
	if (bytes.length > 30 && starts(bytes, RIFF) && starts(bytes, WEBP, 8) && starts(bytes, [0x56, 0x50, 0x38, 0x58], 12)) {
		// VP8X：24 位小端画布尺寸减一
		const w = 1 + (bytes[24] | (bytes[25] << 8) | (bytes[26] << 16))
		const h = 1 + (bytes[27] | (bytes[28] << 8) | (bytes[29] << 16))
		return { w, h }
	}
	return { w: null, h: null }
}
