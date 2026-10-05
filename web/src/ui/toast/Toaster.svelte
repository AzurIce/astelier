<script lang="ts">
	import { dismissToast, TOAST_ICONS, toasts } from './toast.svelte'
	import Icon from '../Icon.svelte'
</script>

{#if toasts.length > 0}
	<div class="ui-toasts" role="region" aria-label="通知">
		{#each toasts as t (t.id)}
			<div class="ui-toast {t.kind}" role="status">
				<span class="t-icon"><Icon name={TOAST_ICONS[t.kind]} size={16} /></span>
				<div class="t-body">
					<div class="t-title">{t.title}</div>
					{#if t.msg}
						<div class="t-msg">{t.msg}</div>
					{/if}
				</div>
				<button type="button" class="t-close" aria-label="关闭" onclick={() => dismissToast(t.id)}>
					<Icon name="x" size={13} />
				</button>
			</div>
		{/each}
	</div>
{/if}
