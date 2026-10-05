// 工作区选择：本地 OPFS 或远端服务 URL。设备级偏好存 localStorage，
// 不参与同步。切换 = 保存待写内容后整页刷新，由 main.ts 按选择重新装配。
export type WorkspaceChoice = { kind: 'opfs' } | { kind: 'http'; baseUrl: string }

const KEY = 'atelier-workspace'

function readChoice(): WorkspaceChoice {
	try {
		const raw = localStorage.getItem(KEY)
		if (raw) {
			const parsed = JSON.parse(raw) as Partial<WorkspaceChoice> | null
			if (parsed?.kind === 'http' && typeof parsed.baseUrl === 'string' && parsed.baseUrl.trim()) {
				return { kind: 'http', baseUrl: parsed.baseUrl.trim().replace(/\/+$/, '') }
			}
			if (parsed?.kind === 'opfs') return { kind: 'opfs' }
		}
	} catch {
		// 损坏的记录按本地处理
	}
	return { kind: 'opfs' }
}

export const workspaceChoice = $state({ current: readChoice() })

export function currentWorkspaceChoice(): WorkspaceChoice {
	return workspaceChoice.current
}

export function saveWorkspaceChoice(choice: WorkspaceChoice): void {
	workspaceChoice.current = choice
	localStorage.setItem(KEY, JSON.stringify(choice))
}

/** 校验远端工作区连通性（读取图列表）；失败抛错给调用方展示 */
export async function validateRemoteWorkspace(baseUrl: string): Promise<void> {
	const base = baseUrl.trim().replace(/\/+$/, '')
	let res: Response
	try {
		res = await fetch(`${base}/api/graphs`)
	} catch {
		throw new Error('无法连接服务（检查地址、网络或服务端 CORS 配置）')
	}
	if (!res.ok) throw new Error(`服务响应异常（HTTP ${res.status}）`)
}

/** 工作区显示名（顶栏指示用） */
export function workspaceLabel(choice: WorkspaceChoice): string {
	return choice.kind === 'opfs' ? '本地工作区' : `远端 · ${choice.baseUrl}`
}
