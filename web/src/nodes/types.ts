import { ClassicPreset, type GetSchemes } from 'rete'
import type { SvelteArea2D } from 'rete-svelte-plugin/5'
import type { NodeTypes } from './classes'

export type Schemes = GetSchemes<
	NodeTypes,
	ClassicPreset.Connection<ClassicPreset.Node, ClassicPreset.Node>
>
export type AreaExtra = SvelteArea2D<Schemes>

/** Connection 泛型繁琐，运行时形状一致；集中一个构造 helper */
export function connect(
	src: ClassicPreset.Node,
	out: string,
	dst: ClassicPreset.Node,
	inp: string,
): Schemes['Connection'] {
	const C = ClassicPreset.Connection as unknown as new (
		a: ClassicPreset.Node,
		b: string,
		c: ClassicPreset.Node,
		d: string,
	) => Schemes['Connection']
	return new C(src, out, dst, inp)
}
