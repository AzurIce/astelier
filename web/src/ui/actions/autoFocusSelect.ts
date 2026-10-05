// Svelte action：挂载即聚焦并全选。
// 动态插入的 <input autofocus> 在浏览器里常被忽略（非初始解析时机），
// 重命名/新建输入框需要可靠地自动聚焦 + 全选现有文本。
export function autoFocusSelect(node: HTMLInputElement) {
	node.focus()
	node.select()
}
