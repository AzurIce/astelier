// 三套皮肤 × light/dark 的完整设计令牌。
// 令牌值经 store.apply() 写入 <html> 的 style 自定义属性（--ui-*），
// 组件样式只引用语义名，皮肤差异全部收敛在这里。

export type Mode = 'light' | 'dark'

export interface TokenSet {
	/** 画布底（编辑器背景） */
	bg: string
	/** 侧栏 / 顶栏 / 浮层底 */
	panel: string
	/** 节点卡片底 */
	card: string
	/** 卡片内嵌控件底（输入框、按钮底） */
	input: string
	/** hover 高亮底 */
	hover: string
	/** 滑轨 / 分隔轨道 */
	track: string
	/** 网格点色 */
	dot: string

	/** 主文本 */
	text: string
	/** 次级文本（标签、说明） */
	dim: string
	/** 弱文本（占位、禁用） */
	faint: string

	/** 强调色（主按钮、选中态、端口高亮） */
	accent: string
	/** 强调色上的文字 */
	accentText: string
	/** 弱强调底（选中底、chip 底） */
	accentWeak: string

	/** 连线基色（默认 / 未按类型着色时） */
	conn: string
	/** 端口类型色：模型 */
	sockModel: string
	/** 端口类型色：文本 */
	sockText: string
	/** 端口类型色：图像 */
	sockImage: string

	danger: string
	warn: string
	ok: string

	/** 节点阴影 */
	shadowNode: string
	/** 浮层阴影（菜单 / 弹层 / toast） */
	shadowFloat: string

	/** 节点标题栏 */
	titleBg: string
	titleBorder: string
	titleFg: string
	titleWeight: string
	titleSize: string

	/** 网格：CSS background-image 值 */
	gridImage: string
	gridSize: string
}

export interface Skin {
	id: string
	name: string
	blurb: string
	/** 预览色板（皮肤切换器里的小圆点） */
	swatch: [string, string, string]
	dark: TokenSet
	light: TokenSet
	/** 密度与形状 */
	shape: {
		rNode: string
		rControl: string
		rMenu: string
		rImg: string
		titlePadY: string
		titlePadX: string
		bodyPad: string
		gap: string
		controlH: string
	}
	/** 正文字号 */
	fontSize: string
	/** 节点内容区的额外氛围（如 Mono 的弱网格） */
	ambient: string
}

const studio: Skin = {
	id: 'studio',
	name: '工作台',
	blurb: '深色优先 · 高密度 · 克制专业',
	swatch: ['#5b8def', '#1b1d23', '#101114'],
	dark: {
		bg: '#101114',
		panel: '#16181d',
		card: '#1b1e24',
		input: '#22262e',
		hover: '#262a32',
		track: '#2e333c',
		dot: '#2a2e36',
		text: '#e8eaef',
		dim: '#9aa0ab',
		faint: '#5d636e',
		accent: '#5b8def',
		accentText: '#ffffff',
		accentWeak: 'rgba(91, 141, 239, 0.16)',
		conn: '#51637f',
		sockModel: '#5b8def',
		sockText: '#d9b13c',
		sockImage: '#46b881',
		danger: '#e5484d',
		warn: '#d9b13c',
		ok: '#46b881',
		shadowNode: '0 1px 2px rgba(0, 0, 0, 0.5), 0 10px 28px rgba(0, 0, 0, 0.32)',
		shadowFloat: '0 18px 56px rgba(0, 0, 0, 0.55)',
		titleBg: '#1f232a',
		titleBorder: 'rgba(255, 255, 255, 0.06)',
		titleFg: '#e8eaef',
		titleWeight: '600',
		titleSize: '12px',
		gridImage: 'radial-gradient(circle, var(--ui-dot) 1px, transparent 1px)',
		gridSize: '24px 24px',
	},
	light: {
		bg: '#f4f5f7',
		panel: '#ffffff',
		card: '#ffffff',
		input: '#f1f2f5',
		hover: '#ebedf1',
		track: '#dfe2e8',
		dot: '#d3d7de',
		text: '#1c1f26',
		dim: '#646b78',
		faint: '#9aa0ab',
		accent: '#3d74e0',
		accentText: '#ffffff',
		accentWeak: 'rgba(61, 116, 224, 0.12)',
		conn: '#8aa8dd',
		sockModel: '#3d74e0',
		sockText: '#b3862d',
		sockImage: '#35a06b',
		danger: '#d33d3d',
		warn: '#a97a0d',
		ok: '#35a06b',
		shadowNode: '0 1px 2px rgba(20, 24, 40, 0.08), 0 8px 24px rgba(20, 24, 40, 0.10)',
		shadowFloat: '0 18px 56px rgba(20, 24, 40, 0.18)',
		titleBg: '#f7f8fa',
		titleBorder: 'rgba(20, 24, 40, 0.08)',
		titleFg: '#1c1f26',
		titleWeight: '600',
		titleSize: '12px',
		gridImage: 'radial-gradient(circle, var(--ui-dot) 1px, transparent 1px)',
		gridSize: '24px 24px',
	},
	shape: {
		rNode: '10px',
		rControl: '6px',
		rMenu: '10px',
		rImg: '8px',
		titlePadY: '6px',
		titlePadX: '10px',
		bodyPad: '8px',
		gap: '6px',
		controlH: '26px',
	},
	fontSize: '12px',
	ambient: 'none',
}

const aurora: Skin = {
	id: 'aurora',
	name: '晨雾',
	blurb: '圆角柔和 · 明亮友好 · 双主题',
	swatch: ['#7048e8', '#ffffff', '#f0ecfb'],
	dark: {
		bg: '#141220',
		panel: '#1d1a2e',
		card: '#242138',
		input: '#2b2743',
		hover: '#332e4e',
		track: '#3b3560',
		dot: '#302a4a',
		text: '#ece9f7',
		dim: '#a9a3c4',
		faint: '#736d92',
		accent: '#8b7bff',
		accentText: '#ffffff',
		accentWeak: 'rgba(139, 123, 255, 0.18)',
		conn: '#6f639e',
		sockModel: '#8b7bff',
		sockText: '#e8b64c',
		sockImage: '#3ecf8e',
		danger: '#f2555a',
		warn: '#e8b64c',
		ok: '#3ecf8e',
		shadowNode: '0 2px 8px rgba(10, 6, 30, 0.5), 0 16px 44px rgba(10, 6, 30, 0.45)',
		shadowFloat: '0 24px 64px rgba(10, 6, 30, 0.6)',
		titleBg: 'rgba(255, 255, 255, 0.04)',
		titleBorder: 'rgba(255, 255, 255, 0.08)',
		titleFg: '#ece9f7',
		titleWeight: '650',
		titleSize: '12.5px',
		gridImage: 'radial-gradient(circle, var(--ui-dot) 1.2px, transparent 1.2px)',
		gridSize: '26px 26px',
	},
	light: {
		bg: '#f4f2fa',
		panel: '#ffffff',
		card: '#ffffff',
		input: '#f3f1fa',
		hover: '#ece9f7',
		track: '#e2def0',
		dot: '#d8d2ea',
		text: '#211d33',
		dim: '#6d6689',
		faint: '#a49dbe',
		accent: '#7048e8',
		accentText: '#ffffff',
		accentWeak: 'rgba(112, 72, 232, 0.12)',
		conn: '#b3a6e0',
		sockModel: '#7048e8',
		sockText: '#c9932a',
		sockImage: '#2f9d68',
		danger: '#d92d20',
		warn: '#b7791f',
		ok: '#2f9d68',
		shadowNode: '0 1px 3px rgba(76, 40, 140, 0.10), 0 12px 32px rgba(76, 40, 140, 0.12)',
		shadowFloat: '0 24px 64px rgba(76, 40, 140, 0.22)',
		titleBg: '#faf9fe',
		titleBorder: 'rgba(76, 40, 140, 0.08)',
		titleFg: '#211d33',
		titleWeight: '650',
		titleSize: '12.5px',
		gridImage: 'radial-gradient(circle, var(--ui-dot) 1.2px, transparent 1.2px)',
		gridSize: '26px 26px',
	},
	shape: {
		rNode: '16px',
		rControl: '10px',
		rMenu: '16px',
		rImg: '12px',
		titlePadY: '8px',
		titlePadX: '12px',
		bodyPad: '12px',
		gap: '8px',
		controlH: '30px',
	},
	fontSize: '13px',
	ambient: 'none',
}

const mono: Skin = {
	id: 'mono',
	name: '素白',
	blurb: '无边框 · 大留白 · 图像优先',
	swatch: ['#1c1c1e', '#ffffff', '#f4f4f2'],
	dark: {
		bg: '#0c0c0d',
		panel: '#131315',
		card: '#171719',
		input: '#202023',
		hover: '#262629',
		track: '#2c2c30',
		dot: '#232326',
		text: '#f2f2f0',
		dim: '#a1a1a0',
		faint: '#6b6b6a',
		accent: '#f2f2f0',
		accentText: '#131315',
		accentWeak: 'rgba(242, 242, 240, 0.10)',
		conn: '#6e6e70',
		sockModel: '#d8d8d6',
		sockText: '#b8b8b4',
		sockImage: '#9a9a96',
		danger: '#ff6363',
		warn: '#ffb864',
		ok: '#4ade80',
		shadowNode: '0 1px 2px rgba(0, 0, 0, 0.6)',
		shadowFloat: '0 24px 80px rgba(0, 0, 0, 0.7)',
		titleBg: 'transparent',
		titleBorder: 'transparent',
		titleFg: '#f2f2f0',
		titleWeight: '650',
		titleSize: '13px',
		gridImage: 'none',
		gridSize: '0',
	},
	light: {
		bg: '#fbfbf9',
		panel: '#ffffff',
		card: '#ffffff',
		input: '#f4f4f1',
		hover: '#efefec',
		track: '#e6e6e2',
		dot: '#e8e8e4',
		text: '#1c1c1e',
		dim: '#77776f',
		faint: '#a8a8a2',
		accent: '#1c1c1e',
		accentText: '#ffffff',
		accentWeak: 'rgba(28, 28, 30, 0.08)',
		conn: '#b3b3ae',
		sockModel: '#1c1c1e',
		sockText: '#6f6f68',
		sockImage: '#9c9c95',
		danger: '#c2312b',
		warn: '#9a6a12',
		ok: '#1a7f37',
		shadowNode: '0 1px 2px rgba(0, 0, 0, 0.04), 0 12px 40px rgba(0, 0, 0, 0.05)',
		shadowFloat: '0 24px 80px rgba(0, 0, 0, 0.14)',
		titleBg: 'transparent',
		titleBorder: 'transparent',
		titleFg: '#1c1c1e',
		titleWeight: '650',
		titleSize: '13px',
		gridImage: 'none',
		gridSize: '0',
	},
	shape: {
		rNode: '20px',
		rControl: '12px',
		rMenu: '16px',
		rImg: '14px',
		titlePadY: '12px',
		titlePadX: '16px',
		bodyPad: '6px',
		gap: '12px',
		controlH: '32px',
	},
	fontSize: '13px',
	ambient: 'radial-gradient(circle at 30% 0%, var(--ui-accent-weak), transparent 60%)',
}

export const SKINS: Skin[] = [studio, aurora, mono]

export function skinById(id: string): Skin {
	return SKINS.find((s) => s.id === id) ?? studio
}
