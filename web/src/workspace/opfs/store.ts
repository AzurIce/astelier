// WorkspaceStore 的 OPFS 实现。目录结构沿用原 Rust 服务 data/ 布局：
//
//   atelier/
//   ├── config.json            Provider 配置（key 明文，设置里可清除）
//   ├── groups.json            图分组
//   ├── graphs/{uuid}/
//   │   ├── graph.json         节点与连线（结构文档）
//   │   ├── view.json          布局与视口（表现文档）
//   │   └── store/             图内参考图
//   └── stores/                用户显式收藏的图片库（层级目录）
//
// 图身份是稳定 UUID：目录名只在建图时生成，重命名只改 graph.json（同步友好）。
// 业务语义（建图种子、分组级联、同名同内容幂等、路径校验）移植自 Rust
// store.rs / api.rs，逐条对齐；磁盘即唯一事实来源，无索引清单。
import type { GraphDoc, GraphDocWithId, GraphGroup, GraphSummary, ProviderConfig, ProviderEntry, StoreFileEntry, ViewDoc, DocNode, DocEdge } from '../types'
import type { WorkspaceStore } from '../store'
import { sniffDimensions } from '../../images/sniff'

import { dirExists, ensureDir, listDir, movePath, readBytes, readFile, readJson, removePath, uuid, withFsLock, writeBytes, writeJson, WORKSPACE_ROOT, type FsPath } from './fs'
import { freeStoreName, resolveStoreDirPath, safeStoreFile, safeStorePath } from './paths'
import { graphStoreObjectUrl, releaseGraphStoreObjectUrls, releaseStoreObjectUrl, storeObjectUrl } from './objectUrls'

interface GraphRecord {
	id: string
	title: string
	group_id: string | null
	nodes: DocNode[]
	edges: DocEdge[]
	created_at: number
	updated_at: number
}

const TREE_LIMIT = 5000
const SNIFF_HEAD = 256 * 1024

const graphDir = (id: string): FsPath => [...WORKSPACE_ROOT, 'graphs', id]
const graphJson = (id: string): FsPath => [...graphDir(id), 'graph.json']
const viewJson = (id: string): FsPath => [...graphDir(id), 'view.json']
const graphStoreDir = (gid: string): FsPath => [...graphDir(gid), 'store']
const graphStoreFile = (gid: string, name: string): FsPath => [...graphStoreDir(gid), name]
const groupsJson = (): FsPath => [...WORKSPACE_ROOT, 'groups.json']
const configJson = (): FsPath => [...WORKSPACE_ROOT, 'config.json']
const storesRoot = (): FsPath => [...WORKSPACE_ROOT, 'stores']

function defaultConfig(): ProviderConfig {
	const models = ['gpt-image-2', 'gpt-image-2.5-sunburst', 'gpt-image-2.5-flare']
	const entry = (id: string, name: string, base_url: string): ProviderEntry => ({ id, name, base_url, api_key: '', models: [...models], overrides: {} })
	return {
		active_provider: 'openai',
		providers: [
			entry('openai', 'OpenAI 官方', 'https://api.openai.com/v1'),
			entry('poke', 'Poke API', 'https://www.poke2api.com/v1'),
		],
	}
}

function sameBytes(a: Uint8Array, b: Uint8Array): boolean {
	return a.length === b.length && a.every((byte, i) => byte === b[i])
}

async function bytesOf(blob: Blob): Promise<Uint8Array<ArrayBuffer>> {
	return new Uint8Array(await blob.arrayBuffer())
}

async function sniffHead(file: File) {
	return sniffDimensions(new Uint8Array(await file.slice(0, SNIFF_HEAD).arrayBuffer()))
}

/** 建图种子：Model→Prompt→Generate→Preview 最小管线（对齐 Rust api.rs） */
function seedGraph(id: string, title: string, groupId: string | null, modelParams: Record<string, string>): { record: GraphRecord; view: ViewDoc } {
	const nid = (tag: string) => `${tag}-${uuid()}`
	const [model, prompt, gen, prev] = ['model', 'prompt', 'gen', 'prev'].map(nid)
	const now = Date.now()
	return {
		record: {
			id,
			title,
			group_id: groupId,
			nodes: [
				{ id: model, type: 'model', params: modelParams },
				{ id: prompt, type: 'prompt', params: { text: 'a cat' } },
				{ id: gen, type: 'generate', params: {} },
				{ id: prev, type: 'preview', params: {} },
			],
			edges: [
				{ id: `${model}:model->${gen}:model`, source: model, sourcePort: 'model', target: gen, targetPort: 'model' },
				{ id: `${prompt}:text->${gen}:prompt`, source: prompt, sourcePort: 'text', target: gen, targetPort: 'prompt' },
				{ id: `${gen}:image->${prev}:image`, source: gen, sourcePort: 'image', target: prev, targetPort: 'image' },
			],
			created_at: now,
			updated_at: now,
		},
		view: {
			version: 1,
			positions: {
				[model]: { x: 60, y: 120 },
				[prompt]: { x: 60, y: 380 },
				[gen]: { x: 420, y: 200 },
				[prev]: { x: 780, y: 200 },
			},
		},
	}
}

async function readGraphRecord(id: string): Promise<GraphRecord | null> {
	const record = await readJson<Partial<GraphRecord>>(graphJson(id))
	if (!record || typeof record.id !== 'string' || !record.id) return null
	return {
		id: record.id,
		title: typeof record.title === 'string' ? record.title : '未命名图',
		group_id: record.group_id ?? null,
		nodes: Array.isArray(record.nodes) ? record.nodes : [],
		edges: Array.isArray(record.edges) ? record.edges : [],
		created_at: record.created_at ?? Date.now(),
		updated_at: record.updated_at ?? Date.now(),
	}
}

function toDocWithId(record: GraphRecord): GraphDocWithId {
	return { version: 1, id: record.id, title: record.title, group_id: record.group_id, nodes: record.nodes, edges: record.edges }
}

async function readGroups(): Promise<GraphGroup[]> {
	const list = await readJson<GraphGroup[]>(groupsJson())
	return Array.isArray(list) ? list : []
}

async function writeGroups(groups: GraphGroup[]): Promise<void> {
	await writeJson(groupsJson(), groups)
}

/** 分组删除的受影响集合：自身 + 全部传递子目录 */
function doomedGroups(groups: GraphGroup[], id: string): Set<string> {
	const doomed = new Set([id])
	for (let grew = true; grew;) {
		grew = false
		for (const g of groups) {
			if (g.parent_id && doomed.has(g.parent_id) && !doomed.has(g.id)) {
				doomed.add(g.id)
				grew = true
			}
		}
	}
	return doomed
}

/** 库全树：递归走目录（磁盘即真相），逐文件读头部嗅探尺寸 */
async function collectTree(dir: FsPath, rel: string, dirs: string[], files: StoreFileEntry[], budget: { left: number }): Promise<void> {
	if (budget.left <= 0) return
	const entries = await listDir(dir)
	for (const name of entries.dirs) {
		if (budget.left-- <= 0) return
		const childRel = rel ? `${rel}/${name}` : name
		dirs.push(childRel)
		await collectTree([...dir, name], childRel, dirs, files, budget)
	}
	for (const name of entries.files) {
		if (name === 'manifest.json') continue
		const safe = safeStoreFile(name)
		if (!safe) continue
		if (budget.left-- <= 0) return
		const file = await readFile([...dir, name])
		const dims = file ? await sniffHead(file) : { w: null, h: null }
		files.push({
			path: rel ? `${rel}/${safe}` : safe,
			...(dims.w != null ? { w: dims.w } : {}),
			...(dims.h != null ? { h: dims.h } : {}),
			bytes: file?.size ?? 0,
		})
	}
}

/** 返回对象是普通字面量（测试会用展开包裹 putGraph/putView 注入故障） */
export function createOpfsStore(): WorkspaceStore {
	/** 无锁内核：调用方已在 withFsLock 内时使用（Web Locks 同名不可重入） */
	async function readConfigUnlocked(): Promise<ProviderConfig> {
		const stored = await readJson<ProviderConfig>(configJson())
		if (stored && Array.isArray(stored.providers) && stored.providers.length) {
			return {
				providers: stored.providers.map((p) => ({
					id: String(p?.id ?? ''),
					name: String(p?.name ?? ''),
					base_url: String(p?.base_url ?? ''),
					api_key: String(p?.api_key ?? ''),
					models: Array.isArray(p?.models) ? p.models.map(String) : [],
					overrides: p?.overrides && typeof p.overrides === 'object' ? p.overrides : {},
				})),
				active_provider: String(stored.active_provider ?? ''),
			}
		}
		const seeded = defaultConfig()
		await writeJson(configJson(), seeded)
		return seeded
	}

	async function saveGraphRecord(record: GraphRecord): Promise<void> {
		await writeJson(graphJson(record.id), record)
	}

	async function putStoreBytes(path: FsPath, bytes: Uint8Array<ArrayBuffer>): Promise<{ w: number | null; h: number | null; bytes: number }> {
		await writeBytes(path, bytes)
		const dims = sniffDimensions(bytes)
		return { w: dims.w, h: dims.h, bytes: bytes.length }
	}

	const store: WorkspaceStore = {
		kind: 'opfs',
		label: '本地工作区（OPFS）',

		async loadConfig() {
			return withFsLock(readConfigUnlocked)
		},

		async saveConfig(config) {
			await writeJson(configJson(), config)
		},

		async listGraphs() {
			const out: GraphSummary[] = []
			for (const name of (await listDir([...WORKSPACE_ROOT, 'graphs'])).dirs) {
				const record = await readGraphRecord(name)
				if (record) out.push({ id: record.id, title: record.title, group_id: record.group_id, updated_at: record.updated_at })
			}
			return out.sort((a, b) => b.updated_at - a.updated_at)
		},

		async createGraph(groupId, title) {
			return withFsLock(async () => {
				if (groupId && !(await readGroups()).some((g) => g.id === groupId)) throw new Error('目标目录不存在')
				const config = await readConfigUnlocked()
				const active = config.providers.find((p) => p.id === config.active_provider) ?? config.providers[0]
				const modelParams = active ? { provider: active.id, modelId: active.models[0] ?? '' } : { provider: '', modelId: '' }
				const id = uuid()
				const { record, view } = seedGraph(id, title?.trim() || '未命名图', groupId, modelParams)
				// view 先落盘：结构文档存在即视为完整图
				await writeJson(viewJson(id), view)
				await saveGraphRecord(record)
				return toDocWithId(record)
			})
		},

		async fetchGraph(id) {
			const record = await readGraphRecord(id)
			if (!record) throw new Error('图不存在')
			return toDocWithId(record)
		},

		async putGraph(id, doc: GraphDoc) {
			await withFsLock(async () => {
				const record = await readGraphRecord(id)
				if (!record) throw new Error('图不存在')
				await saveGraphRecord({ ...record, nodes: doc.nodes, edges: doc.edges, updated_at: Date.now() })
			})
		},

		async renameGraph(id, title) {
			return withFsLock(async () => {
				const record = await readGraphRecord(id)
				if (!record) throw new Error('图不存在')
				await saveGraphRecord({ ...record, title, updated_at: Date.now() })
				return { id }
			})
		},

		async fetchView(id) {
			const view = await readJson<Partial<ViewDoc>>(viewJson(id))
			return {
				version: 1,
				positions: view?.positions ?? {},
				...(view?.viewport ? { viewport: view.viewport } : {}),
			}
		},

		async putView(id, view) {
			await writeJson(viewJson(id), view)
		},

		async setGraphGroup(id, groupId) {
			await withFsLock(async () => {
				const record = await readGraphRecord(id)
				if (!record) throw new Error('图不存在')
				await saveGraphRecord({ ...record, group_id: groupId, updated_at: Date.now() })
			})
		},

		async deleteGraph(id) {
			await removePath(graphDir(id))
			releaseGraphStoreObjectUrls(id)
		},

		async listGroups() {
			return (await readGroups()).sort((a, b) => a.created_at - b.created_at)
		},

		async createGroup(name, parentId) {
			return withFsLock(async () => {
				const groups = await readGroups()
				if (parentId && !groups.some((g) => g.id === parentId)) throw new Error('父目录不存在')
				const now = Date.now()
				const group: GraphGroup = { id: uuid(), name: name.trim() || '新建文件夹', parent_id: parentId, created_at: now, updated_at: now }
				await writeGroups([...groups, group])
				return group
			})
		},

		async renameGroup(id, name) {
			await withFsLock(async () => {
				const groups = await readGroups()
				const slot = groups.find((g) => g.id === id)
				if (!slot) return
				slot.name = name
				slot.updated_at = Date.now()
				await writeGroups(groups)
			})
		},

		async moveGroup(id, parentId) {
			await withFsLock(async () => {
				const groups = await readGroups()
				if (parentId) {
					if (parentId === id) throw new Error('不能把目录移动到它自己内部')
					let cursor = parentId
					const byId = new Map(groups.map((g) => [g.id, g] as const))
					while (cursor) {
						if (cursor === id) throw new Error('不能把目录移动到它自己内部')
						cursor = byId.get(cursor)?.parent_id ?? ''
					}
					if (!groups.some((g) => g.id === parentId)) throw new Error('目标目录不存在')
				}
				const slot = groups.find((g) => g.id === id)
				if (!slot) return
				slot.parent_id = parentId
				slot.updated_at = Date.now()
				await writeGroups(groups)
			})
		},

		async deleteGroup(id) {
			await withFsLock(async () => {
				const groups = await readGroups()
				const doomed = doomedGroups(groups, id)
				await writeGroups(groups.filter((g) => !doomed.has(g.id)))
				// 组内与子组内的图回到未分组（图是资产，不连带删）
				for (const name of (await listDir([...WORKSPACE_ROOT, 'graphs'])).dirs) {
					const record = await readGraphRecord(name)
					if (record?.group_id && doomed.has(record.group_id)) {
						await saveGraphRecord({ ...record, group_id: null })
					}
				}
			})
		},

		async uploadGraphStoreFile(gid, name, blob) {
			return withFsLock(async () => {
				const safe = safeStoreFile(name)
				if (!safe) throw new Error('文件名不合法')
				const bytes = await bytesOf(blob)
				if (!bytes.length) throw new Error('空文件')
				// 同名同内容 → 保持原名（幂等）；同名不同内容 → 加序号，绝不互相覆盖
				let target = safe
				const existing = await readBytes(graphStoreFile(gid, safe))
				if (existing && !sameBytes(existing, bytes)) {
					const taken = (await listDir(graphStoreDir(gid))).files
					target = freeStoreName(safe, (candidate) => taken.has(candidate), uuid)
				}
				const meta = await putStoreBytes(graphStoreFile(gid, target), bytes)
				return { name: target, ...(meta.w != null ? { w: meta.w } : {}), ...(meta.h != null ? { h: meta.h } : {}), bytes: meta.bytes }
			})
		},

		async graphStoreUrl(gid, name) {
			return graphStoreObjectUrl(gid, name)
		},

		async deleteGraphStoreFile(gid, name) {
			const safe = safeStoreFile(name)
			if (!safe || safe !== name) throw new Error('文件名不合法')
			await removePath(graphStoreFile(gid, safe))
			releaseGraphStoreObjectUrls(gid, safe)
		},

		async storeTree() {
			const dirs: string[] = []
			const files: StoreFileEntry[] = []
			await collectTree(storesRoot(), '', dirs, files, { left: TREE_LIMIT })
			return { dirs: dirs.sort(), files: files.sort((a, b) => (a.path < b.path ? -1 : a.path > b.path ? 1 : 0)) }
		},

		async uploadStoreFile(file, dir) {
			const safeDir = dir.trim() ? resolveStoreDirPath(dir) : ''
			if (dir.trim() && !safeDir) throw new Error('目标目录不合法')
			const safe = safeStoreFile(file.name)
			if (!safe) throw new Error('文件名不合法')
			const bytes = await bytesOf(file)
			if (!bytes.length) throw new Error('空文件')
			const rel = safeDir ? `${safeDir}/${safe}` : safe
			const meta = await putStoreBytes([...storesRoot(), ...rel.split('/')], bytes)
			return { path: rel, ...(meta.w != null ? { w: meta.w } : {}), ...(meta.h != null ? { h: meta.h } : {}), bytes: meta.bytes }
		},

		async makeStoreDir(path) {
			const resolved = resolveStoreDirPath(path)
			if (!resolved) throw new Error('目录路径不合法')
			await ensureDir([...storesRoot(), ...resolved.split('/')])
		},

		async moveStorePath(from, to) {
			const safeFrom = resolveStoreDirPath(from)
			if (!safeFrom) throw new Error('源路径不合法')
			const safeTo = resolveStoreDirPath(to)
			if (!safeTo) throw new Error('目标路径不合法')
			if (safeFrom === safeTo) return
			if (safeTo.startsWith(`${safeFrom}/`)) throw new Error('不能移动到自己的子目录')
			if (!(await dirExists([...storesRoot(), ...safeFrom.split('/')]))) throw new Error('源路径不存在')
			await movePath([...storesRoot(), ...safeFrom.split('/')], [...storesRoot(), ...safeTo.split('/')])
			releaseStoreObjectUrl(from)
		},

		async deleteStorePath(path) {
			const rel = safeStorePath(path) ?? resolveStoreDirPath(path)
			if (!rel) throw new Error('路径不合法')
			await removePath([...storesRoot(), ...rel.split('/')])
			releaseStoreObjectUrl(path)
		},

		async storeUrl(path) {
			return storeObjectUrl(path)
		},
	}
	return store
}
