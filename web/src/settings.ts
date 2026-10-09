// 设备本地设置。保持零依赖，供主题、后端注册表和图会话共同使用。
export const SKIN_KEY = 'astelier-skin'
export const MODE_KEY = 'astelier-mode'
export const GRAPH_KEY = 'astelier-active-graph'
export const CONNECTIONS_KEY = 'astelier-backends'
export const SIDEBAR_WIDTH_KEY = 'astelier-sidebar-w'

export function readSetting(key: string): string | null {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage.getItem(key)
	} catch {
		return null
	}
}
