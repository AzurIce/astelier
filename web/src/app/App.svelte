<script lang="ts">
	import { onMount } from 'svelte'
	import Sidebar from '../workspace/Sidebar.svelte'
	import Topbar from './Topbar.svelte'
	import SettingsDialog from './SettingsDialog.svelte'
	import ViewportControls from '../canvas/components/ViewportControls.svelte'
	import NodePalette from '../canvas/components/NodePalette.svelte'
	import Toaster from '../ui/toast/Toaster.svelte'
	import ConfirmHost from '../ui/confirm/ConfirmHost.svelte'
	import LightboxHost from '../ui/lightbox/LightboxHost.svelte'
	import LibraryDock from '../library/LibraryDock.svelte'
	import Icon from '../ui/Icon.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { rt } from '../canvas/runtime'
	import { loadProviderConfig } from '../generation/config.svelte'
	import { currentWorkspaceChoice } from '../workspace/selection.svelte'
	import { activeGraphId, ensureGraphAndLoad, graphSession, renameGraphAndSync } from '../canvas/session.svelte'
	import { createEditor } from '../canvas/editor'
	import { fitView } from '../canvas/viewport'

	let ready = $state(false)
	let running = $state(false)
	let nodeCount = $state(0)
	let dockOpen = $state(true)
	let settingsOpen = $state(false)

	let sidebar: Sidebar | undefined = $state()
	let palette: NodePalette | undefined = $state()

	function refreshStructure() {
		nodeCount = rt.editor?.getNodes().length ?? 0
	}

	async function run() {
		if (running || graphSession.loading) return
		running = true
		try {
			const { runPipeline } = await import('../canvas/execute')
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
		// 本地工作区：一次性申请持久存储，降低站点数据被自动清理的风险
		if (currentWorkspaceChoice().kind === 'opfs') {
			navigator.storage?.persist?.().catch(() => {})
		}
		try {
			await loadProviderConfig()
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
		<Topbar title={graphSession.title} saveState={graphSession.saveState} {running} ready={ready && !graphSession.loading} onRename={renameTitle} onRun={run} onSettings={() => (settingsOpen = true)} />
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
<SettingsDialog open={settingsOpen} onclose={() => (settingsOpen = false)} />
