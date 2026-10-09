// 模型能力档案（移植自 Rust profiles.rs / model.rs，语义保持一致）。
// params schema 是参数区的渲染契约：表单控件、能力徽章、请求体收集、
// 自定义尺寸校验全部由它驱动。接入新模型 = 增加一份档案或由 provider
// override 提供元数据，不改代码。
//
// 本模块是浏览器直连生图的唯一参数来源；服务端 /api/providers/{provider_id}/generate 仍按
// 自己的档案兜底，两侧同源维护。

export type ParamKind = 'select' | 'number' | 'text' | 'size'
export type Mode = 'gen' | 'edit'

/** 统一参数值。unset = 该参数不随请求发送。 */
export type ParamValue =
	| { t: 'unset' }
	| { t: 'text'; v: string }
	| { t: 'number'; v: number }
	| { t: 'size'; v: string }

export type ParamMap = Record<string, ParamValue>

export interface ParamDef {
	key: string
	/** 发送到请求体的字段名；空串 = 同 key */
	api_key: string
	label: string
	kind: ParamKind
	options: string[]
	min: number | null
	max: number | null
	/** 归入「更多参数」折叠区 */
	advanced: boolean
	/** 语义分组：'' = 生成主参数，'output' = 输出，'safety' = 审核与标识 */
	group: string
	/** 限定出现的模式；空 = gen + edit 都出现 */
	modes: Mode[]
	/** 协议默认值：缺失该键时按此值显式发送（null = 不强制） */
	default_value: ParamValue | null
}

export interface SizeRule {
	step: number
	min_side: number
	max_w: number
	max_h: number
	ratio_min: number
	ratio_max: number
	note: string
}

export interface RatioPreset {
	label: string
	w: number
	h: number
}

export type ApiKind = 'openai_images' | 'generic'

export interface ModelProfile {
	id: string
	label: string
	api: ApiKind
	stream: boolean
	max_prompt: number
	max_refs: number
	mask_edit: boolean
	transparent: boolean
	edit_note: string
	params: ParamDef[]
	size_rule: SizeRule | null
	/** 比例预设（空 = 退回扁平 presets 列表） */
	size_ratios: RatioPreset[]
}

export function findParam(profile: ModelProfile, key: string): ParamDef | undefined {
	return profile.params.find((d) => d.key === key)
}

export function apiKeyOf(def: ParamDef): string {
	return def.api_key || def.key
}

// ---------- 内置档案 ----------

function param(key: string, label: string, kind: ParamKind): ParamDef {
	return { key, api_key: '', label, kind, options: [], min: null, max: null, advanced: false, group: '', modes: [], default_value: null }
}

const withOptions = (p: ParamDef, options: string[]): ParamDef => ({ ...p, options })
/** 默认值 = options 第一项（OpenAI Images 协议里各枚举的默认档） */
const firstOption = (p: ParamDef): ParamDef =>
	p.default_value === null && p.options.length ? { ...p, default_value: { t: 'text', v: p.options[0] } } : p
const withMax = (p: ParamDef, max: number): ParamDef => ({ ...p, min: 1, max })
const advanced = (p: ParamDef): ParamDef => ({ ...p, advanced: true })
const inGroup = (p: ParamDef, group: string): ParamDef => ({ ...p, group })
const editOnly = (p: ParamDef): ParamDef => ({ ...p, modes: ['edit'] })

export function qualityDef(opts: string[]): ParamDef {
	return firstOption(withOptions(param('quality', '画质', 'select'), opts))
}

export function sizeDef(presets: string[]): ParamDef {
	return firstOption(withOptions(param('size', '尺寸', 'size'), presets))
}

export function nDef(max: number): ParamDef {
	return { ...withMax(param('n', '数量', 'number'), max), default_value: { t: 'number', v: 1 } }
}

export function backgroundDef(): ParamDef {
	return firstOption(withOptions(param('background', '背景', 'select'), ['auto', 'transparent', 'opaque']))
}

export function moderationDef(): ParamDef {
	return firstOption(advanced(inGroup(withOptions(param('moderation', '审核', 'select'), ['auto', 'low']), 'safety')))
}

export function outputFormatDef(): ParamDef {
	return firstOption(inGroup(withOptions(param('output_format', '输出格式', 'select'), ['png', 'jpeg', 'webp']), 'output'))
}

export function outputCompressionDef(): ParamDef {
	return { ...advanced(inGroup(withMax(param('output_compression', '压缩率', 'number'), 100), 'output')), default_value: { t: 'number', v: 100 } }
}

export function inputFidelityDef(): ParamDef {
	return firstOption(editOnly(inGroup(withOptions(param('input_fidelity', '原图保真', 'select'), ['low', 'high']), 'safety')))
}

const SIZE_PRESETS = ['auto', '1024x1024', '1536x1024', '1024x1536']

function gptParams(qualityOpts: string[]): ParamDef[] {
	return [
		qualityDef(qualityOpts),
		sizeDef(SIZE_PRESETS),
		nDef(10),
		backgroundDef(),
		outputFormatDef(),
		moderationDef(),
		outputCompressionDef(),
		inputFidelityDef(),
	]
}

export function gptImageSizeRule(): SizeRule {
	return {
		step: 16,
		min_side: 16,
		max_w: 3840,
		max_h: 2160,
		ratio_min: 1 / 3,
		ratio_max: 3,
		note: '边长需被 16 整除 · 宽高比 1:3–3:1 · 上限 3840×2160（超 2560×1440 为实验性）',
	}
}

function baseProfile(id: string, label: string, params: ParamDef[]): ModelProfile {
	return {
		id,
		label,
		api: 'openai_images',
		stream: true,
		max_prompt: 32000,
		max_refs: 16,
		mask_edit: true,
		transparent: true,
		edit_note: '支持：mask + 最多 16 张参考图 + input_fidelity（high/low）',
		params,
		size_rule: gptImageSizeRule(),
		size_ratios: [],
	}
}

function gptImageSizeRatios(): RatioPreset[] {
	return [
		{ label: '1:1', w: 1024, h: 1024 },
		{ label: '3:2', w: 1536, h: 1024 },
		{ label: '2:3', w: 1024, h: 1536 },
		{ label: '16:9', w: 1792, h: 1008 },
		{ label: '9:16', w: 1008, h: 1792 },
		{ label: '4:3', w: 1360, h: 1024 },
		{ label: '3:4', w: 1024, h: 1360 },
	]
}

/** 按 model id 解析内置档案（provider override 可整体/部分覆盖） */
export function builtinProfile(modelId: string): ModelProfile {
	if (modelId.startsWith('gpt-image-2.5')) {
		return {
			...baseProfile(modelId, 'gpt-image-2.5：画质独占 xhigh / max；透明背景完整支持；任意分辨率（16 整除 · 比例 1:3–3:1 · 上限 3840×2160）', gptParams(['auto', 'high', 'medium', 'low', 'xhigh', 'max'])),
			size_ratios: gptImageSizeRatios(),
		}
	}
	if (modelId.startsWith('gpt-image-2')) {
		return {
			...baseProfile(modelId, 'gpt-image-2：任意分辨率（16 整除 · 比例 1:3–3:1 · 上限 3840×2160）；透明背景为预览特性', gptParams(['auto', 'high', 'medium', 'low'])),
			size_ratios: gptImageSizeRatios(),
		}
	}
	if (modelId.startsWith('gpt-image')) {
		return { ...baseProfile(modelId, 'gpt-image-1 系：固定三档尺寸 + auto；quality high/medium/low；恒返 b64_json', gptParams(['auto', 'high', 'medium', 'low'])), size_rule: null }
	}
	// 未识别模型：按通用 OpenAI 兼容协议放行常用参数，实际以网关为准
	return {
		...baseProfile(modelId, '未识别模型：参数以 Provider / 网关为准，发送前请自行确认', [
			qualityDef(['auto', 'high', 'medium', 'low']),
			sizeDef(SIZE_PRESETS),
			nDef(10),
			backgroundDef(),
			outputFormatDef(),
		]),
		api: 'generic',
		stream: false,
		size_rule: null,
	}
}

// ---------- override 合并 ----------

/**
 * 合并 provider 的元数据覆盖（深合并：对象递归、null 删键、其余整体替换）。
 * 合并结果结构不合法时回退内置档案，对应 Rust 侧「serde 解析失败回退」。
 */
export function mergedProfile(modelId: string, override: unknown): ModelProfile {
	const base = builtinProfile(modelId)
	if (override === null || override === undefined) return base
	const merged: unknown = deepMerge(base as unknown as Record<string, unknown>, override)
	return parseProfile(merged) ?? base
}

export function deepMerge<T>(base: T, ov: unknown): unknown {
	if (isPlainObject(base) && isPlainObject(ov)) {
		const out: Record<string, unknown> = { ...base }
		for (const [k, v] of Object.entries(ov)) {
			if (v === null) delete out[k]
			else out[k] = deepMerge(out[k], v)
		}
		return out
	}
	return ov
}

function isPlainObject(v: unknown): v is Record<string, unknown> {
	return typeof v === 'object' && v !== null && !Array.isArray(v)
}

function parseParamDef(v: unknown): ParamDef | null {
	if (!isPlainObject(v) || typeof v.key !== 'string' || typeof v.label !== 'string') return null
	if (v.kind !== 'select' && v.kind !== 'number' && v.kind !== 'text' && v.kind !== 'size') return null
	if (v.options !== undefined && !Array.isArray(v.options)) return null
	if (v.modes !== undefined && !Array.isArray(v.modes)) return null
	if (v.modes !== undefined && !(v.modes as unknown[]).every((m) => m === 'gen' || m === 'edit')) return null
	return {
		key: v.key,
		api_key: typeof v.api_key === 'string' ? v.api_key : '',
		label: v.label,
		kind: v.kind,
		options: Array.isArray(v.options) ? (v.options as unknown[]).filter((o): o is string => typeof o === 'string') : [],
		min: typeof v.min === 'number' ? v.min : null,
		max: typeof v.max === 'number' ? v.max : null,
		advanced: v.advanced === true,
		group: typeof v.group === 'string' ? v.group : '',
		modes: Array.isArray(v.modes) ? (v.modes as Mode[]) : [],
		default_value: v.default_value == null ? null : parseParamValue(v.default_value),
	}
}

function parseParamValue(v: unknown): ParamValue | null {
	if (!isPlainObject(v)) return null
	if (v.t === 'unset') return { t: 'unset' }
	if (v.t === 'text' && typeof v.v === 'string') return { t: 'text', v: v.v }
	if (v.t === 'number' && typeof v.v === 'number' && Number.isFinite(v.v)) return { t: 'number', v: v.v }
	if (v.t === 'size' && typeof v.v === 'string') return { t: 'size', v: v.v }
	return null
}

function parseProfile(v: unknown): ModelProfile | null {
	if (!isPlainObject(v) || typeof v.id !== 'string') return null
	if (!Array.isArray(v.params)) return null
	const params: ParamDef[] = []
	for (const item of v.params) {
		const def = parseParamDef(item)
		if (!def) return null
		params.push(def)
	}
	const rule = v.size_rule == null ? null : parseSizeRule(v.size_rule)
	if (v.size_rule != null && rule === null) return null
	return {
		id: v.id,
		label: typeof v.label === 'string' ? v.label : '',
		api: v.api === 'generic' ? 'generic' : 'openai_images',
		stream: v.stream === true,
		max_prompt: typeof v.max_prompt === 'number' ? v.max_prompt : 32000,
		max_refs: typeof v.max_refs === 'number' ? v.max_refs : 16,
		mask_edit: v.mask_edit === true,
		transparent: v.transparent === true,
		edit_note: typeof v.edit_note === 'string' ? v.edit_note : '',
		params,
		size_rule: rule,
		size_ratios: Array.isArray(v.size_ratios)
			? (v.size_ratios as unknown[]).filter(isRatioPreset).map((r) => ({ label: r.label, w: r.w, h: r.h }))
			: [],
	}
}

function parseSizeRule(v: unknown): SizeRule | null {
	if (!isPlainObject(v)) return null
	const nums = [v.step, v.min_side, v.max_w, v.max_h, v.ratio_min, v.ratio_max]
	if (!nums.every((n) => typeof n === 'number' && Number.isFinite(n))) return null
	return {
		step: v.step as number,
		min_side: v.min_side as number,
		max_w: v.max_w as number,
		max_h: v.max_h as number,
		ratio_min: v.ratio_min as number,
		ratio_max: v.ratio_max as number,
		note: typeof v.note === 'string' ? v.note : '',
	}
}

function isRatioPreset(v: unknown): v is RatioPreset {
	return isPlainObject(v) && typeof v.label === 'string' && typeof v.w === 'number' && typeof v.h === 'number'
}

// ---------- 默认值与校验 ----------

/** JSON 标量 → ParamValue（对象/数组/null 丢弃），对应服务端 /api/providers/{provider_id}/generate 的参数收窄 */
export function coerceParamValue(v: unknown): ParamValue | null {
	if (typeof v === 'string') return { t: 'text', v }
	if (typeof v === 'number') return Number.isFinite(v) ? { t: 'number', v } : null
	if (typeof v === 'boolean') return { t: 'text', v: String(v) }
	return null
}

/**
 * 补齐协议默认参数（「始终完整发送」）：档案里带 default_value 而请求
 * 缺失的键按默认值显式加入；显式值优先。text 类（default=null）不补。
 */
export function withDefaults(profile: ModelProfile, params: ParamMap): ParamMap {
	const out: ParamMap = { ...params }
	for (const def of profile.params) {
		if (def.key in out) continue
		if (def.default_value && def.default_value.t !== 'unset') out[def.key] = def.default_value
	}
	return out
}

/** 请求级校验：模型已选、图片总数上限、参数取值（按档案）。返回错误文案或 null。 */
export function validateRequest(profile: ModelProfile, modelId: string, params: ParamMap, imageCount: number): string | null {
	if (!modelId) return '请先选择模型'
	if (imageCount > profile.max_refs) return `图片总数超过上限 ${profile.max_refs} 张`
	for (const [key, value] of Object.entries(params)) {
		const def = findParam(profile, key)
		if (!def) continue
		if (def.max !== null && value.t === 'number') {
			if (value.v > def.max || value.v < (def.min ?? Number.NEGATIVE_INFINITY)) return `「${def.label}」超出范围`
		}
		if (def.kind === 'select' && value.t === 'text' && !def.options.includes(value.v)) {
			return `「${def.label}」的取值 ${value.v} 不在可选列表`
		}
	}
	return null
}

/** ParamValue → 请求体 JSON 值（unset → null，由调用方跳过） */
export function toRequestValue(v: ParamValue): string | number | null {
	switch (v.t) {
		case 'unset': return null
		case 'text': return v.v
		case 'size': return v.v
		case 'number': return v.v
	}
}

/**
 * 统一参数映射为协议请求体字段：unset 跳过；档案内键按 api_key 映射；
 * 未识别键原样透传 —— 协议是开集合，能力差异以网关实际响应为准。
 */
export function paramsToBody(profile: ModelProfile, params: ParamMap): Record<string, string | number> {
	const body: Record<string, string | number> = {}
	for (const key of Object.keys(params).sort()) {
		const value = toRequestValue(params[key])
		if (value === null) continue
		const def = findParam(profile, key)
		body[def ? apiKeyOf(def) : key] = value
	}
	return body
}
