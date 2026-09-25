// 运行时共享单例：组件与执行器都要拿 editor/area 实例，
// 用独立模块承载避免 editor ↔ components 循环依赖。
// （普通 .ts：跨模块可变状态经函数/回调通信，不依赖响应式。）
import type { NodeEditor } from 'rete'
import type { AreaPlugin } from 'rete-area-plugin'
import type { Schemes, AreaExtra } from './nodes/types'

/** 正在执行生成的节点 id 集合（连线流动动画用） */
export const runningNodes = new Set<string>()

export const rt: {
	editor?: NodeEditor<Schemes>
	area?: AreaPlugin<Schemes, AreaExtra>
	providers: { id: string; name: string; models: string[] }[]
	activeProvider?: string
	/** 画布右键 → NodePalette（editor.ts 只管事件，不碰 UI 组件） */
	onCanvasContextMenu?: (clientX: number, clientY: number) => void
	/** Ctrl/Cmd + Enter 发起执行 */
	onRunRequested?: () => void
	/** 结构变化（节点/连线增删）—— 供外壳刷新空画布提示等 */
	onStructureChange?: () => void
} = { providers: [] }
