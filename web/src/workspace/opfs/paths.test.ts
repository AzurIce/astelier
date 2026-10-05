import { test } from 'node:test'
import assert from 'node:assert/strict'
import { freeStoreName, resolveStoreDirPath, safeStoreFile, safeStorePath, sanitizeDirName } from './paths.ts'

test('safeStoreFile 允许 Unicode 词干、归一扩展名，拒绝穿越与非图片扩展名', () => {
	assert.equal(safeStoreFile('猫.png'), '猫.png')
	assert.equal(safeStoreFile('x.PNG'), 'x.png')
	assert.equal(safeStoreFile('x.jpeg'), 'x.jpg')
	assert.equal(safeStoreFile('a:b*.png'), 'ab.png')
	assert.equal(safeStoreFile('  猫 .png'), '猫.png')
	assert.equal(safeStoreFile('../evil.png'), null)
	assert.equal(safeStoreFile('a/b.png'), null)
	assert.equal(safeStoreFile('a\\b.png'), null)
	assert.equal(safeStoreFile('no-ext'), null)
	assert.equal(safeStoreFile('.png'), null)
	assert.equal(safeStoreFile('a.svg'), null)
})

test('safeStorePath 允许层级中文目录，拒绝穿越 / 绝对路径 / 空段 / 超深 / 坏扩展名', () => {
	assert.equal(safeStorePath('角色/猫.png'), '角色/猫.png')
	assert.equal(safeStorePath('a/b/c.png'), 'a/b/c.png')
	assert.equal(safeStorePath('猫.png'), '猫.png')
	assert.equal(safeStorePath('  a/b.png  '), 'a/b.png')
	assert.equal(safeStorePath('../x.png'), null)
	assert.equal(safeStorePath('a/../x.png'), null)
	assert.equal(safeStorePath('/abs/x.png'), null)
	assert.equal(safeStorePath('a//x.png'), null)
	assert.equal(safeStorePath('a/b/c/d/e/f/g/h/i.png'), null)
	assert.equal(safeStorePath('a/x.svg'), null)
	assert.equal(safeStorePath('a/b:x/c.png'), null)
	assert.equal(safeStorePath(''), null)
	assert.equal(safeStorePath('a'.repeat(241) + '.png'), null)
})

test('resolveStoreDirPath 校验目录段（末段无扩展名要求）', () => {
	assert.equal(resolveStoreDirPath('角色/猫'), '角色/猫')
	assert.equal(resolveStoreDirPath('a'), 'a')
	assert.equal(resolveStoreDirPath(' a/b '), 'a/b')
	assert.equal(resolveStoreDirPath(''), null)
	assert.equal(resolveStoreDirPath('a//b'), null)
	assert.equal(resolveStoreDirPath('..'), null)
	assert.equal(resolveStoreDirPath('a/b:c'), null)
	assert.equal(resolveStoreDirPath('1/2/3/4/5/6/7/8/9'), null)
})

test('sanitizeDirName 替换非法字符、折叠空白、限长、去首尾点、空回退默认名', () => {
	assert.equal(sanitizeDirName('a/b\\c:d*e?f"g<h>i|j'), 'a-b-c-d-e-f-g-h-i-j')
	assert.equal(sanitizeDirName('  多   空白  '), '多 空白')
	assert.equal(sanitizeDirName('...去点...'), '去点')
	assert.equal(sanitizeDirName('   ...   '), '未命名图')
	assert.equal(sanitizeDirName(''), '未命名图')
	const long = sanitizeDirName('画'.repeat(80))
	assert.equal(Array.from(long).length, 64)
	assert.equal(sanitizeDirName('猫'.repeat(70) + '.'), '猫'.repeat(64))
})

test('freeStoreName 依次尝试 -2/-3，全占用时用兜底 id', () => {
	const taken = (candidate: string) => ['dup.png', 'dup-2.png'].includes(candidate)
	assert.equal(freeStoreName('dup.png', taken, () => 'deadbeef'), 'dup-3.png')
	const all = () => true
	assert.equal(freeStoreName('dup.png', all, () => 'deadbeef'), 'dup-deadbeef.png')
	assert.equal(freeStoreName('noext', () => false, () => 'x'), 'noext-2')
})
