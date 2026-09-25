<script lang="ts">
	import Icon from './Icon.svelte'
	import type { Snippet } from 'svelte'

	// 居中确认对话框（危险操作）。Esc=取消，Enter=确认，点遮罩=取消。
	let {
		title,
		message,
		confirmText = '确认',
		cancelText = '取消',
		danger = false,
		onclose,
		children,
	}: {
		title: string
		message?: string
		confirmText?: string
		cancelText?: string
		danger?: boolean
		onclose?: (ok: boolean) => void
		children?: Snippet
	} = $props()

	let okBtn: HTMLButtonElement | null = $state(null)

	$effect(() => {
		okBtn?.focus()
	})
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
	class="ui-overlay"
	role="presentation"
	onclick={(e) => {
		if (e.target === e.currentTarget) onclose?.(false)
	}}
	onkeydown={(e) => {
		if (e.key === 'Escape') onclose?.(false)
		if (e.key === 'Enter') onclose?.(true)
	}}
>
	<div class="ui-dialog" role="alertdialog" aria-modal="true" aria-label={title}>
		<header>
			{#if danger}<Icon name="alert" size={17} />{/if}
			{title}
		</header>
		{#if message}
			<div class="body">
				{@html message}
			</div>
		{/if}
		{#if children}
			<div class="body">
				{@render children()}
			</div>
		{/if}
		<footer>
			<button type="button" class="ui-btn ghost" onclick={() => onclose?.(false)}>{cancelText}</button>
			<button
				type="button"
				bind:this={okBtn}
				class="ui-btn {danger ? 'primary danger-btn' : 'primary'}"
				onclick={() => onclose?.(true)}>{confirmText}</button>
		</footer>
	</div>
</div>

<style>
	:global(.danger-btn) {
		background: var(--ui-danger);
		color: #fff;
	}
</style>
