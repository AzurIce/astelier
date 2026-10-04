<script lang="ts">
	import { onMount } from 'svelte'
	import Sidebar from './Sidebar.svelte'
	import Topbar from './chrome/Topbar.svelte'
	import ViewportControls from './chrome/ViewportControls.svelte'
	import NodePalette from './NodePalette.svelte'
	import Toaster from './components/Toaster.svelte'
	import ConfirmHost from './components/ConfirmHost.svelte'
	import LightboxHost from './components/LightboxHost.svelte'
	import LibraryDock from './components/LibraryDock.svelte'
	import Icon from './components/Icon.svelte'
	import { toast } from './components/toast.svelte'
	import { rt } from './runtime'
	import { fetchConfig, type ProviderInfo } from './api'
	import { activeGraphId, ensureGraphAndLoad, graphSession, renameGraphAndSync } from './graphStore.svelte'
	import { createEditor, fitView } from './editor'

	let ready = $state(false)
	let running = $state(false)
	let nodeCount = $state(0)
	let dockOpen = $state(true)

	let sidebar: Sidebar | undefined = $state()
	let palette: NodePalette | undefined = $state()

	function refreshStructure() {
		nodeCount = rt.editor?.getNodes().length ?? 0
	}

	async function run() {
		if (running || graphSession.loading) return
		running = true
		try {
			const { runPipeline } = await import('./exec')
			const generated = await runPipeline()
			toast({
				kind: 'ok',
				title: '执行完成',
				msg: generated > 0 ? `完成 ${generated} 次生成` : '图上没有 Generate 节点',
			})
		} catch (e) {
			toast({
				kind: 'err',
				title: '执行失败',
				msg: e instanceof Error ? e.message : String(e),
			})
		}
		running = false
	}

	async function renameTitle(next: string) {
		try {
			await renameGraphAndSync(activeGraphId(), next)
			await sidebar?.refreshAndKeepActive()
		} catch (e) {
			toast({ kind: 'err', title: '重命名失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	onMount(async () => {
		try {
			const cfg = await fetchConfig()
			rt.providers = cfg.providers as ProviderInfo[]
			rt.activeProvider = cfg.active_provider
		} catch (e) {
			toast({ kind: 'err', title: '读取配置失败', msg: e instanceof Error ? e.message : String(e) })
		}

		createEditor(document.getElementById('rete')!)
		rt.onCanvasContextMenu = (cx, cy) => void palette?.openAt(cx, cy)
		rt.onRunRequested = () => void run()
		rt.onStructureChange = refreshStructure

		try {
			await ensureGraphAndLoad()
			// 侧栏的 onMount 早于此处（子组件先挂载），当时图还没载入、
			// activeId 读为空；载入后刷新一次才能高亮当前图
			await sidebar?.refreshAndKeepActive()
		} catch (e) {
			toast({ kind: 'err', title: '载入画布失败', msg: e instanceof Error ? e.message : String(e) })
		}
		refreshStructure()
		// 首访种子图自动适应视野
		requestAnimationFrame(() => fitView())
		ready = true
	})
</script>

<div class="shell">
	<Sidebar bind:this={sidebar} />
	<div class="main">
		<Topbar title={graphSession.title} saveState={graphSession.saveState} {running} ready={ready && !graphSession.loading} onRename={renameTitle} onRun={run} />
		<div class="canvas-layer" inert={graphSession.loading}>
			<div id="rete"></div>
			{#if ready && nodeCount === 0}
				<div class="empty-canvas">
					<div class="empty-card">
						<span class="icon-wrap"><Icon name="layers" size={22} /></span>
						<h2>开始搭建你的生成管线</h2>
						<p>
							在画布上<strong>右键</strong>打开节点面板，
							从 Model、Prompt 走到 Generate 与 Preview。
						</p>
					</div>
				</div>
			{/if}
			<ViewportControls />
		</div>
	</div>
</div>

<LibraryDock collapsed={!dockOpen} ontoggle={() => (dockOpen = !dockOpen)} />

<NodePalette bind:this={palette} />
<Toaster />
<ConfirmHost />
<LightboxHost />
