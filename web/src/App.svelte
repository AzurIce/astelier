<script lang="ts">
	import { onMount } from 'svelte'
	import ContextMenu from './ContextMenu.svelte'
	import { currentTheme, toggleTheme, type Theme } from './theme'
	import { rt } from './runtime'

	let ready = false
	let running = false
	let status = ''
	let error: string | null = null
	let theme: Theme = 'dark'
	let ctxMenu: ContextMenu

	onMount(async () => {
		theme = currentTheme()
		// svelte 插件仅客户端可用，动态导入编辑器模块
		const { fetchConfig } = await import('./api')
		const { createEditor } = await import('./editor')
		const { ensureGraphAndLoad } = await import('./graphStore')

		try {
			const cfg = await fetchConfig()
			rt.providers = cfg.providers
			rt.activeProvider = cfg.active_provider
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		}

		createEditor(document.getElementById('rete')!)
		rt.onCanvasContextMenu = (cx, cy) => ctxMenu.openAt(cx, cy)
		try {
			await ensureGraphAndLoad()
		} catch (e) {
			error = e instanceof Error ? e.message : String(e)
		}
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
		if (!confirm('清空当前画布（服务端将新建一张种子图）？')) return
		localStorage.removeItem('atelier-graph-id')
		location.reload()
	}
</script>

<div class="topbar">
	<div class="brand">Atelier</div>
	<button class="run" disabled={!ready || running} on:click={run}>
		{running ? 'Running…' : 'Run'}
	</button>
	{#if status}<span class="status">{status}</span>{/if}
	{#if error}<span class="error" title={error}>{error}</span>{/if}
	<div class="spacer"></div>
	<button class="ghost" on:click={clearCanvas}>清空</button>
	<button class="ghost icon" title="切换 light / dark" on:click={() => (theme = toggleTheme())}>
		{theme === 'dark' ? '☀' : '☾'}
	</button>
	<span class="hint">右键画布添加节点</span>
</div>
<div id="rete"></div>
<ContextMenu bind:this={ctxMenu} />

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
		font-weight: 650;
		font-size: 13px;
		letter-spacing: 0.4px;
	}
	.spacer {
		flex: 1;
	}
	.hint {
		font-size: 12px;
		color: var(--an-dim);
	}
	.ghost.icon {
		width: 30px;
		padding: 4px 0;
		text-align: center;
	}
</style>
