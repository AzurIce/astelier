<script lang="ts">
	import Icon from './Icon.svelte'
	import type { IconName } from './icons'

	// 图标按钮：全部交互统一走它，保证 hover/active/aria 一致
	let {
		icon,
		label,
		size = 16,
		sm = false,
		active = false,
		danger = false,
		variant = 'subtle',
		onclick,
	}: {
		icon: IconName
		label: string
		size?: number
		sm?: boolean
		active?: boolean
		danger?: boolean
		variant?: 'subtle' | 'ghost'
		onclick?: (e: MouseEvent) => void
	} = $props()
</script>

<button
	type="button"
	class="ui-btn icon {variant}"
	class:active
	class:sm
	class:danger
	aria-label={label}
	aria-pressed={active || undefined}
	title={label}
	{onclick}
>
	<Icon name={icon} {size} />
</button>

<style>
	.ui-btn.icon.active {
		background: var(--ui-accent-weak);
		color: var(--ui-accent);
	}
	.ui-btn.icon.danger:hover {
		background: color-mix(in srgb, var(--ui-danger) 14%, transparent);
		color: var(--ui-danger);
	}
</style>
