<script lang="ts">
	import { ICONS, type IconName } from '../design/icons'

	// 线性图标容器：所有描边走 currentColor，线宽/尺寸可在组件内覆写
	let {
		name,
		size = 16,
		strokeWidth = 1.7,
		class: cls = '',
	}: { name: IconName; size?: number; strokeWidth?: number; class?: string } = $props()
</script>

<svg
	class="ui-icon {cls}"
	width={size}
	height={size}
	viewBox="0 0 24 24"
	fill="none"
	stroke="currentColor"
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-hidden="true"
	focusable="false"
>
	{@html ICONS[name]}
</svg>

<style>
	.ui-icon {
		display: block;
		flex: none;
	}
</style>
