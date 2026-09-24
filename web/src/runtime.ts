// 运行时共享单例：组件与执行器都要拿 editor/area 实例，
// 用独立模块承载避免 editor ↔ components 循环依赖。
import { writable } from 'svelte/store'
import type { NodeEditor } from 'rete'
import type { AreaPlugin } from 'rete-area-plugin'
import type { Schemes, AreaExtra } from './nodes/types'

export const rt: {
	editor?: NodeEditor<Schemes>
	area?: AreaPlugin<Schemes, AreaExtra>
	providers: { id: string; name: string; models: string[] }[]
	activeProvider?: string
} = { providers: [] }

/** Model 节点选择 / 连线变化时自增；Generate 据此重取 profiles */
export const modelVersion = writable(0)
export function bumpModel() {
	modelVersion.update((n) => n + 1)
}
