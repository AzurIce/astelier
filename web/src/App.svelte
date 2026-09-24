<script lang="ts">
	import { onMount } from 'svelte'

	let ready = false
	let running = false
	let status = ''
	let error: string | null = null

	onMount(async () => {
		// svelte 插件仅客户端可用，动态导入编辑器模块
		const { fetchConfig } = await import('./api')
		const { rt } = await import('./runtime')
		const { createEditor } = await import('./editor')
		const { restoreGraph, scheduleSave } = await import('./persist')
		const { seedGraph } = await import('./seed')

		try {
			const cfg = await fetchConfig()
			rt.providers = cfg.providers
			rt.activeProvider = cfg.active_provider
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		}

		createEditor(document.getElementById('rete')!)
		const restored = await restoreGraph()
		if (!restored) await seedGraph()
		scheduleSave()
		ready = true
	})

	async function run() {
		if (running) return
		running = true
		error = null
		status = '执行中…'
		try {
			const { runPipeline } = await import('./exec')
			const generated = await runPipeline()
			status = `完成（${generated} 次生成）`
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
			status = ''
		}
		running = false
	}

	function clearCanvas() {
		if (!confirm('清空当前画布？')) return
		localStorage.removeItem('atelier-graph-v1')
		location.reload()
	}
</script>

<div class="topbar">
	<div class="brand">Atelier</div>
	<button class="run" disabled={!ready || running} on:click={run}>
		{running ? 'Running…' : '▶ Run'}
	</button>
		{#if status}<span class="status">{status}</span>{/if}
		{#if error}<span class="error" title={error}>{error}</span>{/if}
		<div class="spacer"></div>
	<button class="ghost" on:click={clearCanvas}>清空</button>
	<span class="hint">右键画布添加节点</span>
</div>
<div id="rete"></div>

<style>
	.topbar {
		display: flex;
		align-items: center;
		gap: 12px;
		height: 44px;
		padding: 0 14px;
		border-bottom: 1px solid var(--an-border);
		background: var(--an-panel);
	}
	.brand {
		font-weight: 700;
		letter-spacing: 0.5px;
	}
	.spacer {
		flex: 1;
	}
	.hint {
		font-size: 12px;
		opacity: 0.5;
	}
</style>
