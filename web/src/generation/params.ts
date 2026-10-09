// OpenAI Images 协议参数表（gpt-image 系）。
// 由 generation/profiles.ts 的内置档案派生——参数定义单一事实来源，
// 直连生图（默认补齐/校验/请求映射）与参数区渲染读同一份；
// 这里只补充纯展示层信息：控件形态（control）与尺寸候选扩展。
import { builtinProfile, type ModelProfile } from './profiles'

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
	/** 协议默认值（与档案 default_value 同源）：始终完整发送 */
	def: string | number | null
}

/** UI 控件形态（纯展示偏好，协议层不感知） */
const CONTROL_BY_KEY: Record<string, 'slider' | 'segmented'> = {
	quality: 'slider',
	background: 'segmented',
	output_format: 'segmented',
}

/** gpt-image-2+ 常用比例参考档（16:9 / 9:16），追加到档案预设之后 */
const EXTRA_SIZE_PRESETS = ['1792x1008', '1008x1792']

function defaultValue(v: unknown): string | number | null {
	if (v == null || typeof v !== 'object') return null
	const value = v as { t?: string; v?: unknown }
	if (value.t === 'text' || value.t === 'size') return typeof value.v === 'string' ? value.v : null
	if (value.t === 'number') return typeof value.v === 'number' ? value.v : null
	return null
}

export function profileParams(profile: ModelProfile): ParamDef[] { return profile.params.map((def) => ({
	key: def.key,
	label: def.label,
	kind: def.kind,
	control: CONTROL_BY_KEY[def.key],
	options: def.key === 'size' && profile.size_rule ? [...new Set([...def.options, ...EXTRA_SIZE_PRESETS])] : def.options,
	min: def.min,
	max: def.max,
	advanced: def.advanced,
	group: def.group,
	def: defaultValue(def.default_value),
})) }

export const OPENAI_IMAGE_PARAMS = profileParams(builtinProfile('gpt-image-2'))
