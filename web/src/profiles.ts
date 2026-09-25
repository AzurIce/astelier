// 模型档案（params schema）：按 provider 拉取并缓存。
// schema 是节点参数区的渲染契约 —— 与服务端 profiles.rs 同源。

export interface ParamDef {
	key: string
	label: string
	kind: 'select' | 'number' | 'text' | 'size'
	/** select 类的控件形态：默认下拉；slider=离散档位滑块；segmented=按钮组 */
	control?: 'slider' | 'segmented'
	options: string[]
	min: number | null
	max: number | null
	advanced: boolean
	group: string
}

export interface ModelProfile {
	id: string
	label: string
	params: ParamDef[]
}

const cache = new Map<string, ModelProfile[]>()

export async function fetchProfiles(providerId: string): Promise<ModelProfile[]> {
	const hit = cache.get(providerId)
	if (hit) return hit
	const res = await fetch(`/api/providers/${encodeURIComponent(providerId)}/profiles`)
	if (!res.ok) throw new Error(`读取模型档案失败（${res.status}）`)
	const profiles = (await res.json()) as ModelProfile[]
	cache.set(providerId, profiles)
	return profiles
}
