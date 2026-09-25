// 节点相关的小动作：删除级联、上传回写等，集中一处避免循环依赖
// （editor.ts 的删除快捷键也走这里）。
import { rt } from '../runtime'
import { connKeys } from './conn'

/** 删除节点并级联清理相邻连线 */
export async function removeNodeCascade(nodeId: string): Promise<void> {
	const editor = rt.editor
	if (!editor) return
	for (const conn of editor.getConnections()) {
		const k = connKeys(conn as unknown as Record<string, unknown>)
		if (k.source === nodeId || k.target === nodeId) {
			await editor.removeConnection(conn.id)
		}
	}
	await editor.removeNode(nodeId)
}
