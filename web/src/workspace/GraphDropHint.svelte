<script lang="ts">
	import Icon from '../ui/Icon.svelte'
	import type { GraphDropPreview } from './graphDrop'
	let { preview }: { preview: GraphDropPreview } = $props()
</script>

<div class="graph-drop-hint" class:unavailable={!preview.allowed} role="status">
	<span class="hint-icon"><Icon name={preview.allowed ? 'graph' : 'alert'} size={20} /></span>
	<div class="hint-copy">
		<strong>{preview.allowed ? '松手导入图包' : '暂时无法导入'}</strong>
		<span class="destination">{preview.backendName} › {preview.folderName}</span>
		<small>{preview.allowed ? preview.fileLabel : '存储离线或正在处理图包'}</small>
	</div>
</div>

<style>
	.graph-drop-hint { display: flex; align-items: center; gap: 10px; padding: 12px; border: 1px dashed var(--ui-accent); border-radius: var(--ui-r-menu); background: color-mix(in srgb, var(--ui-accent) 8%, var(--ui-panel)); color: var(--ui-text); pointer-events: none; }
	.hint-icon { display: grid; place-items: center; flex: none; width: 36px; height: 36px; border-radius: var(--ui-r-control); color: var(--ui-accent); background: var(--ui-accent-weak); }
	.hint-copy { display: flex; flex-direction: column; gap: 4px; min-width: 0; }
	strong { font-size: 12px; }
	.destination { font-size: 11px; overflow-wrap: anywhere; }
	small { font-size: 10px; color: var(--ui-dim); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.unavailable { border-color: var(--ui-warn); }
	.unavailable .hint-icon { color: var(--ui-warn); }
</style>
