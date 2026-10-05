// Provider 配置与共享响应式状态。
export interface ProviderInfo {
	id: string
	name: string
	models: string[]
}

export const providerConfig = $state({
	providers: [] as ProviderInfo[],
})

export async function loadProviderConfig(): Promise<void> {
	const res = await fetch('/api/config')
	if (!res.ok) throw new Error(`读取配置失败（${res.status}）`)
	const config = await res.json()
	providerConfig.providers = config.providers
}

