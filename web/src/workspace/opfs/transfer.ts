// 工作区导入/导出：把整个 OPFS 工作区（astelier/）打包为 zip 下载，
// 或从 zip 合并导入，用于换设备/换域名迁移和完整备份。
// 单图可移植包由 workspace/graphArchive.ts 管理。
//
// 合并规则（对齐未来同步的 last-writer-wins）：
// - 图：按 id 对应本地目录；本地较新（updated_at 更大）则跳过，否则整目录覆盖
// - 分组：按 id 并集，冲突保留本地
// - 库文件：字节不同则覆盖（路径即身份）
// - Provider 配置不导入（密钥是设备本地数据，用户在新设备重新填写）
import { unzipSync, zipSync } from 'fflate'
import { listDir, readJson, readBytes, withFsLock, writeBytes, writeJson, ensureDir, WORKSPACE_ROOT, type FsPath } from './fs'
import type { GraphGroup } from '../types'
import { releaseGraphStoreObjectUrls, releaseStoreObjectUrl } from './objectUrls'
import { downloadFile } from '../../ui/download'

const GRAPH_DIR = /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/

// ---------- 纯逻辑（单测覆盖） ----------

export interface ImportEntry {
	path: string
	bytes: Uint8Array<ArrayBuffer>
}

export interface GraphBundle {
	id: string
	graph?: ImportEntry
	view?: ImportEntry
	store: ImportEntry[]
}

export interface ImportPlan {
	graphs: GraphBundle[]
	library: ImportEntry[]
	directories: string[]
	groups?: ImportEntry
	rejected: string[]
}

/** zip 条目路径校验与归类；接受带或不带 astelier/ 前缀。 */
export function planImport(entries: ImportEntry[]): ImportPlan {
	const plan: ImportPlan = { graphs: [], library: [], directories: [], rejected: [] }
	const bundles = new Map<string, GraphBundle>()
	for (const entry of entries) {
		const directory = entry.path.endsWith('/')
		const segs = safeSegments(directory ? entry.path.slice(0, -1) : entry.path)
		if (!segs) {
			plan.rejected.push(entry.path)
			continue
		}
		const rel = segs[0] === WORKSPACE_ROOT[0] ? segs.slice(1) : segs
		if (directory) {
			if (rel[0] === 'stores' && rel.length > 1) plan.directories.push(rel.slice(1).join('/'))
			continue
		}
		if (rel.length === 1 && rel[0] === 'groups.json') {
			plan.groups = entry
			continue
		}
		if (rel[0] === 'graphs' && rel.length >= 3 && GRAPH_DIR.test(rel[1])) {
			const id = rel[1]
			let bundle = bundles.get(id)
			if (!bundle) {
				bundle = { id, store: [] }
				bundles.set(id, bundle)
			}
			if (rel.length === 3 && rel[2] === 'graph.json') bundle.graph = entry
			else if (rel.length === 3 && rel[2] === 'view.json') bundle.view = entry
			else if (rel.length === 4 && rel[2] === 'store') bundle.store.push({ ...entry, path: rel[3] })
			else plan.rejected.push(entry.path)
			continue
		}
		if (rel[0] === 'stores' && rel.length >= 2) {
			plan.library.push({ ...entry, path: rel.slice(1).join('/') })
			continue
		}
		// config.json 不导入；runs/assets/legacy 等历史目录拒绝
		plan.rejected.push(entry.path)
	}
	plan.graphs = [...bundles.values()].filter((b) => b.graph)
	return plan
}

function safeSegments(path: string): string[] | null {
	if (!path || path.startsWith('/') || path.includes('\\') || path.includes(':')) return null
	const segs = path.split('/')
	if (segs.length > 12) return null
	for (const seg of segs) {
		if (!seg || seg === '.' || seg === '..' || /[*?"<>|]/.test(seg)) return null
	}
	return segs
}

/** 图合并决策：本地不存在或本地较旧 → 导入（restore / last-writer-wins） */
export function graphTake(localUpdatedAt: number | null, importedUpdatedAt: number | null): boolean {
	if (localUpdatedAt === null) return true
	if (importedUpdatedAt === null) return false
	return importedUpdatedAt >= localUpdatedAt
}

/** 分组并集：按 id，冲突保留本地 */
export function mergeGroups(local: GraphGroup[], imported: GraphGroup[]): GraphGroup[] {
	const ids = new Set(local.map((g) => g.id))
	return [...local, ...imported.filter((g) => !ids.has(g.id))]
}

function updatedAtOf(entry: ImportEntry): number | null {
	try {
		const parsed = JSON.parse(new TextDecoder().decode(entry.bytes)) as { updated_at?: unknown }
		return typeof parsed.updated_at === 'number' ? parsed.updated_at : null
	} catch {
		return null
	}
}

// ---------- OPFS 操作 ----------

async function collectDir(dir: FsPath, prefix: string, out: Record<string, Uint8Array>): Promise<void> {
	const entries = await listDir(dir)
	for (const name of entries.dirs) { out[`${prefix}${name}/`] = new Uint8Array(); await collectDir([...dir, name], `${prefix}${name}/`, out) }
	for (const name of entries.files) {
		if (dir.length === WORKSPACE_ROOT.length && name === 'config.json') continue
		const bytes = await readBytes([...dir, name])
		if (bytes) out[`${prefix}${name}`] = bytes
	}
}

export async function exportWorkspaceZip(): Promise<string> {
	const files: Record<string, Uint8Array> = {}
	await withFsLock(() => collectDir(WORKSPACE_ROOT, 'astelier/', files))
	const zipped = zipSync(files)
	const name = `astelier-workspace-${new Date().toISOString().slice(0, 19).replaceAll(/[:T]/g, '-')}.zip`
	downloadFile(new File([zipped], name, { type: 'application/zip' }))
	return name
}

export interface ImportReport {
	graphsTaken: number
	graphsSkipped: number
	libraryFiles: number
	groupsMerged: boolean
	rejected: number
}

export async function importWorkspaceZip(file: File): Promise<ImportReport> {
	// fflate 解出的 Uint8Array 由全新 ArrayBuffer 背书
	const plan = planImport(Object.entries(unzipSync(new Uint8Array(await file.arrayBuffer()))).map(([path, bytes]) => ({ path, bytes: bytes as Uint8Array<ArrayBuffer> })))
	for (const bundle of plan.graphs) {
		let graph: { id?: unknown; nodes?: unknown; edges?: unknown }
		try { graph = JSON.parse(new TextDecoder().decode(bundle.graph!.bytes)) } catch { throw new Error(`图备份无法解析：${bundle.id}`) }
		if (graph?.id !== bundle.id || !Array.isArray(graph.nodes) || !Array.isArray(graph.edges)) throw new Error(`图备份格式不正确：${bundle.id}`)
	}
	const report: ImportReport = { graphsTaken: 0, graphsSkipped: 0, libraryFiles: plan.library.length, groupsMerged: false, rejected: plan.rejected.length }

	await withFsLock(async () => {
		for (const directory of plan.directories) await ensureDir([...WORKSPACE_ROOT, 'stores', ...directory.split('/')])
		for (const bundle of plan.graphs) {
			const local = await readJson<{ updated_at?: unknown }>([...WORKSPACE_ROOT, 'graphs', bundle.id, 'graph.json'])
			const localAt = typeof local?.updated_at === 'number' ? local.updated_at : null
			if (!graphTake(localAt, updatedAtOf(bundle.graph!))) {
				report.graphsSkipped++
				continue
			}
			if (bundle.view) await writeBytes([...WORKSPACE_ROOT, 'graphs', bundle.id, 'view.json'], bundle.view.bytes)
			for (const entry of bundle.store) await writeBytes([...WORKSPACE_ROOT, 'graphs', bundle.id, 'store', entry.path], entry.bytes)
			await writeBytes([...WORKSPACE_ROOT, 'graphs', bundle.id, 'graph.json'], bundle.graph!.bytes)
			releaseGraphStoreObjectUrls(bundle.id)
			report.graphsTaken++
		}
		for (const entry of plan.library) {
			const target: FsPath = [...WORKSPACE_ROOT, 'stores', ...entry.path.split('/')]
			const existing = await readBytes(target)
			if (!existing || !sameBytes(existing, entry.bytes)) {
				await writeBytes(target, entry.bytes)
				releaseStoreObjectUrl(entry.path)
			}
		}
		if (plan.groups) {
			let importedGroups: unknown
			try { importedGroups = JSON.parse(new TextDecoder().decode(plan.groups.bytes)) }
			catch { importedGroups = null }
			if (Array.isArray(importedGroups)) {
				const localGroups = await readJson<GraphGroup[]>([...WORKSPACE_ROOT, 'groups.json'])
				const merged = mergeGroups(Array.isArray(localGroups) ? localGroups : [], importedGroups)
				await writeJson([...WORKSPACE_ROOT, 'groups.json'], merged)
				report.groupsMerged = true
			}
		}

	})
	return report
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
	return a.length === b.length && a.every((byte, i) => byte === b[i])
}
