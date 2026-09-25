<script lang="ts">
	import { closeLightbox, lightbox } from './lightbox.svelte'
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && closeLightbox()} />

{#if lightbox.url}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_click_events_have_key_events -->
	<div
		class="ui-lightbox"
		role="dialog"
		aria-label="图片预览"
		tabindex="-1"
		onclick={() => closeLightbox()}
	>
		<img src={lightbox.url} alt="预览大图" />
	</div>
{/if}
