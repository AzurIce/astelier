import { test } from 'node:test'
import assert from 'node:assert/strict'
import { sniffDimensions, sniffExt, imageMime } from './sniff.ts'

const be32 = (n: number): number[] => [(n >>> 24) & 255, (n >>> 16) & 255, (n >>> 8) & 255, n & 255]

function pngWith(w: number, h: number): Uint8Array {
	const bytes = new Uint8Array(29)
	bytes.set([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 0x0d, 0x49, 0x48, 0x44, 0x52])
	bytes.set(be32(w), 16)
	bytes.set(be32(h), 20)
	return bytes
}

function jpegWith(w: number, h: number): Uint8Array {
	const bytes = new Uint8Array(30)
	bytes.set([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10], 0) // APP0 段长 16
	bytes[20] = 0xff
	bytes[21] = 0xc0 // SOF0
	bytes.set([0x00, 0x11], 22)
	bytes[24] = 8
	bytes[25] = (h >> 8) & 255
	bytes[26] = h & 255
	bytes[27] = (w >> 8) & 255
	bytes[28] = w & 255
	return bytes
}

function webpWith(w: number, h: number): Uint8Array {
	const bytes = new Uint8Array(31)
	bytes.set([0x52, 0x49, 0x46, 0x46, 0, 0, 0, 0, 0x57, 0x45, 0x42, 0x50], 0)
	bytes.set([0x56, 0x50, 0x38, 0x58, 0, 0, 0, 0, 0, 0, 0, 0], 12) // VP8X 头
	const le24 = (n: number): number[] => [n & 255, (n >> 8) & 255, (n >> 16) & 255]
	bytes.set(le24(w - 1), 24)
	bytes.set(le24(h - 1), 27)
	return bytes
}

test('sniffExt 识别 PNG / JPEG / WebP / GIF，未知签名按 PNG', () => {
	assert.equal(sniffExt(pngWith(1, 1)), 'png')
	assert.equal(sniffExt(jpegWith(1, 1)), 'jpg')
	assert.equal(sniffExt(webpWith(1, 1)), 'webp')
	assert.equal(sniffExt(new Uint8Array([0x47, 0x49, 0x46, 0x38, 0x37, 0x61])), 'gif')
	assert.equal(sniffExt(new Uint8Array([0x68, 0x65, 0x6c, 0x6c, 0x6f])), 'png')
})

test('sniffDimensions 解析 PNG IHDR / JPEG SOF / WebP VP8X 宽高', () => {
	assert.deepEqual(sniffDimensions(pngWith(1580, 996)), { w: 1580, h: 996 })
	assert.deepEqual(sniffDimensions(jpegWith(1920, 1080)), { w: 1920, h: 1080 })
	assert.deepEqual(sniffDimensions(webpWith(1024, 768)), { w: 1024, h: 768 })
})

test('sniffDimensions 对 GIF 与未知格式返回 null，头部截断不抛错', () => {
	assert.deepEqual(sniffDimensions(new Uint8Array([0x47, 0x49, 0x46, 0x38])), { w: null, h: null })
	assert.deepEqual(sniffDimensions(new Uint8Array(4)), { w: null, h: null })
	assert.deepEqual(sniffDimensions(new Uint8Array(0)), { w: null, h: null })
	// JPEG 段长越界时安全退出
	const broken = jpegWith(1, 1)
	broken.set([0xff, 0xd8, 0xff, 0xe0, 0xff, 0xff], 0)
	assert.deepEqual(sniffDimensions(broken), { w: null, h: null })
})

test('imageMime 按扩展名给 MIME', () => {
	assert.equal(imageMime('png'), 'image/png')
	assert.equal(imageMime('jpg'), 'image/jpeg')
	assert.equal(imageMime('jpeg'), 'image/jpeg')
	assert.equal(imageMime('webp'), 'image/webp')
	assert.equal(imageMime('gif'), 'image/gif')
	assert.equal(imageMime('other'), 'image/png')
})
