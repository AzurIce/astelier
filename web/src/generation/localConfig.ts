// ---------- Provider 配置（config.json） ----------

export interface ProviderEntry {
	id: string
	name: string
	/** 含 /v1 的根地址，如 https://api.openai.com/v1 */
	base_url: string
	/** 本地模式明文保存（可在设置里清除）；远端模式留在服务端 */
	api_key: string
	models: string[]
	/** 模型档案覆盖（model_id → ModelProfile 子集） */
	overrides: Record<string, unknown>
}

export interface ProviderConfig {
	providers: ProviderEntry[]
	active_provider: string
}


import { readJson, writeJson, withFsLock, WORKSPACE_ROOT } from '../workspace/opfs/fs'

const path = [...WORKSPACE_ROOT, 'config.json']

/** Called inside the OPFS write lock when creating a graph. */
export async function readLocalConfigUnlocked(): Promise<ProviderConfig> {
	const config = await readJson<ProviderConfig>(path)
	if (config && Array.isArray(config.providers)) return config
	const defaults: ProviderConfig = {
		active_provider: 'openai',
		providers: [
			{ id: 'openai', name: 'OpenAI', base_url: 'https://api.openai.com/v1', api_key: '', models: ['gpt-image-2'], overrides: {} },
			{ id: 'poke', name: 'Poke', base_url: 'https://www.poke2api.com/v1', api_key: '', models: ['gpt-image-2'], overrides: {} },
		],
	}
	await writeJson(path, defaults)
	return defaults
}

export function loadLocalConfig(): Promise<ProviderConfig> {
	return withFsLock(readLocalConfigUnlocked)
}

export async function saveLocalConfig(config: ProviderConfig): Promise<void> {
	const ids = config.providers.map((p) => p.id)
	if (ids.some((id) => !id.trim()) || new Set(ids).size !== ids.length) throw new Error('Provider ID 必须非空且唯一')
	await withFsLock(() => writeJson(path, config))
}
