// Rete 连线实例或文档形状 → 节点 ID 与端口键。
export function connKeys(c: Record<string, unknown>) {
	const nodeId = (v: unknown) =>
		typeof v === 'string' ? v : String((v as { id?: string })?.id ?? '')
	return {
		source: nodeId(c.source),
		target: nodeId(c.target),
		output: String(c.sourceOutput ?? c.output ?? ''),
		input: String(c.targetInput ?? c.input ?? ''),
	}
}

