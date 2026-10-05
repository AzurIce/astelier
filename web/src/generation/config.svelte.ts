// Provider 配置与共享响应式状态。数据来自当前工作区 store；
// 这里只保留 Model 节点需要的字段（凭证不进响应式状态）。
import { workspaceStore } from '../workspace/store'

export interface ProviderInfo {
	id: string
	name: string
	models: string[]
}

export const providerConfig = $state({
	providers: [] as ProviderInfo[],
})

export async function loadProviderConfig(): Promise<void> {
	const config = await workspaceStore().loadConfig()
	providerConfig.providers = config.providers.map((p) => ({ id: p.id, name: p.name, models: p.models }))
}
