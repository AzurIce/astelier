<script lang="ts">
	import { confirmState, resolveConfirm } from './confirm.svelte'
	import ConfirmDialog from './ConfirmDialog.svelte'
</script>

{#if confirmState.req}
	{@const req = confirmState.req}
	<ConfirmDialog
		title={req.title}
		message={req.message}
		confirmText={req.confirmText}
		cancelText={req.cancelText}
		danger={req.danger}
		onclose={(ok: boolean) => resolveConfirm(ok)}
	/>
{/if}
