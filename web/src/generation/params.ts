// OpenAI Images 协议参数表（gpt-image 系）。
// 与后端 src/profiles.rs 的档案同源（当前为镜像维护；权威默认值在后端
// ParamDef.default_value，前端展示与「始终完整发送」init 以这里为准，
// 后端生成接口 也会按档案兜底补齐）。
// UI 不再提供「默认」占位：每个参数始终有具体值，缺失键按 def 归一。
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
	/** 协议默认值（与后端 ParamDef.default_value 同源）：
	 *  始终完整发送——控件初值与缺失键归一都以它为准 */
	def: string | number | null
}

export const OPENAI_IMAGE_PARAMS: ParamDef[] = [
	{
		key: 'quality',
		label: '画质',
		kind: 'select',
		control: 'slider',
		options: ['auto', 'high', 'medium', 'low', 'xhigh', 'max'],
		min: null,
		max: null,
		advanced: false,
		group: '',
		def: 'auto',
	},
	{
		key: 'size',
		label: '尺寸',
		kind: 'size',
		// 常用预设建议；gpt-image-2+ 支持任意 16 整除的自定义 WxH，可自由输入
		options: ['auto', '1024x1024', '1536x1024', '1024x1536', '1792x1008', '1008x1792'],
		min: null,
		max: null,
		advanced: false,
		group: '',
		def: 'auto',
	},
	{
		key: 'n',
		label: '数量',
		kind: 'number',
		options: [],
		min: 1,
		max: 10,
		advanced: false,
		group: '',
		def: 1,
	},
	{
		key: 'background',
		label: '背景',
		kind: 'select',
		control: 'segmented',
		options: ['auto', 'transparent', 'opaque'],
		min: null,
		max: null,
		advanced: false,
		group: '',
		def: 'auto',
	},
	{
		key: 'output_format',
		label: '输出格式',
		kind: 'select',
		control: 'segmented',
		options: ['png', 'jpeg', 'webp'],
		min: null,
		max: null,
		advanced: false,
		group: 'output',
		def: 'png',
	},
	{
		key: 'input_fidelity',
		label: '原图保真',
		kind: 'select',
		options: ['low', 'high'],
		min: null,
		max: null,
		advanced: true,
		group: 'edit',
		def: 'low',
	},
	{
		key: 'output_compression',
		label: '压缩率',
		kind: 'number',
		options: [],
		min: 0,
		max: 100,
		advanced: true,
		group: 'output',
		def: 100,
	},
	{
		key: 'moderation',
		label: '审核',
		kind: 'select',
		options: ['auto', 'low'],
		min: null,
		max: null,
		advanced: true,
		group: 'safety',
		def: 'auto',
	},
]