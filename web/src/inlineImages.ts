// 节点 inline 图片的浏览器本地存储（IndexedDB）。
// 「不保存」语义：手动上传 / 生成结果只在浏览器本地，graph.json 只存
// { source:'inline', w, h, mime } 标记；显式拖进 image store 才落服务端。
// 代价：换设备 / 清站点数据会丢——这是「不保存」的诚实代价。

const DB = 'atelier-inline'
const STORE = 'images'

function open(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const req = indexedDB.open(DB, 1)
		req.onupgradeneeded = () => {
			req.result.createObjectStore(STORE)
		}
		req.onsuccess = () => resolve(req.result)
		req.onerror = () => reject(req.error)
	})
}

/** key = graph:{gid}:node:{nid} */
export function inlineKey(gid: string, nodeId: string): string {
	return `graph:${gid}:node:${nodeId}`
}

export async function inlinePut(key: string, blob: Blob): Promise<void> {
	const db = await open()
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readwrite')
		tx.objectStore(STORE).put(blob, key)
		tx.oncomplete = () => resolve()
		tx.onerror = () => reject(tx.error)
	})
}

export async function inlineGet(key: string): Promise<Blob | null> {
	const db = await open()
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readonly')
		const req = tx.objectStore(STORE).get(key)
		req.onsuccess = () => resolve((req.result as Blob) ?? null)
		req.onerror = () => reject(req.error)
	})
}

export async function inlineDel(key: string): Promise<void> {
	const db = await open()
	return new Promise((resolve, reject) => {
		const tx = db.transaction(STORE, 'readwrite')
		tx.objectStore(STORE).delete(key)
		tx.oncomplete = () => resolve()
		tx.onerror = () => reject(req_error(tx))
	})
}

function req_error(tx: IDBTransaction): unknown {
	return tx.error
}

/** Blob → data URL（传给 /api/generate；服务端 url_to_asset 已支持） */
export async function blobToDataUrl(blob: Blob): Promise<string> {
	return new Promise((resolve, reject) => {
		const fr = new FileReader()
		fr.onload = () => resolve(String(fr.result))
		fr.onerror = () => reject(fr.error)
		fr.readAsDataURL(blob)
	})
}
