// 皮肤 / 明暗模式的响应式单例（Svelte 5 runes，跨组件共享）。
// 用法：组件里 `import { design } from '../design/store.svelte'`，
// 读 `design.skin` / `design.mode` / `design.tokens`，切 `design.setSkin(id)` /
// `design.toggleMode()`。切换即写入 CSS 自定义属性（--ui-*）。
import { skinById, type Mode, type Skin, type TokenSet } from './tokens'

const SKIN_KEY = 'atelier-skin'
const MODE_KEY = 'atelier-mode'

function readInitialSkin(): Skin {
	if (typeof localStorage === 'undefined') return skinById('studio')
	// ?skin=aurora 临时覆盖（调试 / 分享外观，不落盘）
	const q = new URLSearchParams(location.search).get('skin')
	if (q) return skinById(q)
	return skinById(localStorage.getItem(SKIN_KEY) ?? '')
}

function readInitialMode(): Mode {
	if (typeof localStorage === 'undefined') return 'dark'
	const q = new URLSearchParams(location.search).get('mode')
	if (q === 'light' || q === 'dark') return q
	const stored = localStorage.getItem(MODE_KEY)
	if (stored === 'light' || stored === 'dark') return stored
	return window.matchMedia?.('(prefers-color-scheme: light)').matches ? 'light' : 'dark'
}

let skin = $state<Skin>(readInitialSkin())
let mode = $state<Mode>(readInitialMode())

/** 语义令牌的扁平注入表：CSS 变量名（--ui-* 去掉前缀）→ 值 */
function flatten(t: TokenSet, s: Skin): Record<string, string> {
	return {
		bg: t.bg,
		panel: t.panel,
		card: t.card,
		input: t.input,
		hover: t.hover,
		track: t.track,
		dot: t.dot,
		text: t.text,
		dim: t.dim,
		faint: t.faint,
		accent: t.accent,
		'accent-text': t.accentText,
		'accent-weak': t.accentWeak,
		conn: t.conn,
		'sock-model': t.sockModel,
		'sock-text': t.sockText,
		'sock-image': t.sockImage,
		danger: t.danger,
		warn: t.warn,
		ok: t.ok,
		'shadow-node': t.shadowNode,
		'shadow-float': t.shadowFloat,
		'title-bg': t.titleBg,
		'title-border': t.titleBorder,
		'title-fg': t.titleFg,
		'title-weight': t.titleWeight,
		'title-size': t.titleSize,
		'grid-image': t.gridImage,
		'grid-size': t.gridSize,
		ambient: s.ambient,
		'r-node': s.shape.rNode,
		'r-control': s.shape.rControl,
		'r-menu': s.shape.rMenu,
		'r-img': s.shape.rImg,
		'title-pad-y': s.shape.titlePadY,
		'title-pad-x': s.shape.titlePadX,
		'body-pad': s.shape.bodyPad,
		gap: s.shape.gap,
		'control-h': s.shape.controlH,
		'font-size': s.fontSize,
	}
}

const PREFIX = '--ui-'

/** 把当前 skin × mode 的令牌写进 <html> 内联样式 */
export function applyDesign(): void {
	const t = skin[mode]
	const vars = flatten(t, skin)
	const el = document.documentElement
	for (const [key, value] of Object.entries(vars)) {
		el.style.setProperty(PREFIX + key, value)
	}
	el.dataset.skin = skin.id
	el.dataset.theme = mode
	el.style.colorScheme = mode
	// 派生量：组件直接引用
	el.style.setProperty(PREFIX + 'accent-fade', `color-mix(in srgb, var(${PREFIX}accent) 44%, transparent)`)
	el.style.setProperty(PREFIX + 'border-fade', `color-mix(in srgb, var(${PREFIX}text) 10%, transparent)`)
}

function persist(): void {
	localStorage.setItem(SKIN_KEY, skin.id)
	localStorage.setItem(MODE_KEY, mode)
}

export const design = {
	get skin(): Skin {
		return skin
	},
	get mode(): Mode {
		return mode
	},
	get tokens(): TokenSet {
		return skin[mode]
	},
	setSkin(id: string) {
		const next = skinById(id)
		if (next.id === skin.id) return
		skin = next
		persist()
		applyDesign()
	},
	setMode(next: Mode) {
		if (next === mode) return
		mode = next
		persist()
		applyDesign()
	},
	toggleMode() {
		this.setMode(mode === 'dark' ? 'light' : 'dark')
	},
}

// 用户未显式选过明暗时，跟随系统
if (typeof window !== 'undefined') {
	window.matchMedia?.('(prefers-color-scheme: light)').addEventListener?.('change', (e) => {
		if (localStorage.getItem(MODE_KEY)) return
		design.setMode(e.matches ? 'light' : 'dark')
	})
}

applyDesign()
