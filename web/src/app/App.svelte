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
	import { backendRegistry, backend, backendStore, connectBackend } from '../backends/registry.svelte'
	import { activeGraph, ensureGraphAndLoad, graphSession, renameGraphAndSync, flushNow, openGraph } from '../canvas/session.svelte'
	import { createGraphArchive, importGraphArchive } from '../workspace/graphArchive'
	import { downloadFile } from '../ui/download'
	import { createEditor } from '../canvas/editor'
	import { fitView } from '../canvas/viewport'
	import { readSetting, SIDEBAR_COLLAPSED_KEY } from '../settings'
	import GraphDropHint from '../workspace/GraphDropHint.svelte'
	import { graphDropDestination, isGraphArchiveDrag, type GraphDropPreview, type GraphImportDestination } from '../workspace/graphDrop'

	let ready = $state(false)
	let running = $state(false)
	let nodeCount = $state(0)
	let dockOpen = $state(true)
	let settingsOpen = $state(false)
	let sidebarOpen = $state(readSetting(SIDEBAR_COLLAPSED_KEY) !== 'true')
	function toggleSidebar() { sidebarOpen = !sidebarOpen; localStorage.setItem(SIDEBAR_COLLAPSED_KEY, String(!sidebarOpen)) }
	let archiveHover: GraphDropPreview | null = $state(null)
	let transferringGraph = $state(false)
	let graphImportInput: HTMLInputElement | undefined = $state()
	let importDestination: GraphImportDestination = { backendId: 'local', groupId: null }

	async function exportGraph() {
		if (transferringGraph) return
		const location = activeGraph(), store = backendStore(location.backendId)
		transferringGraph = true
		try { await flushNow(); downloadFile(await createGraphArchive(store, location.id)); toast({ kind: 'ok', title: '图包已导出' }) }
		catch (error) { toast({ kind: 'err', title: '导出图失败', msg: String(error) }) }
		finally { transferringGraph = false }
	}
	function chooseGraphImport() {
		importDestination = { backendId: graphSession.backendId, groupId: null }
		graphImportInput?.click()
	}
	async function importGraphs(files: File[], destination: GraphImportDestination) {
		if (!files.length || transferringGraph || !ready) return
		if (backend(destination.backendId).status !== 'online') { toast({ kind: 'err', title: '无法导入图包', msg: '目标存储当前离线，请连接后重试' }); return }
		const store = backendStore(destination.backendId), sourceName = backend(destination.backendId).name
		transferringGraph = true
		try {
			await flushNow()
			let lastId = '', imported = 0
			for (const file of files) {
				try {
					const graph = await importGraphArchive(store, file, destination.groupId)
					lastId = graph.id; imported++
					backend(destination.backendId).revision++
				} catch (error) { toast({ kind: 'err', title: `导入失败：${file.name}`, msg: String(error) }) }
			}
			if (lastId) { await openGraph({ backendId: destination.backendId, id: lastId }); toast({ kind: 'ok', title: `已导入 ${imported} 张图`, msg: `保存在「${sourceName}」` }) }
		} catch (error) { toast({ kind: 'err', title: '导入图失败', msg: String(error) }) }
		finally { transferringGraph = false }
	}
	onMount(() => {
		const clear = () => { archiveHover = null }
		const over = (e: DragEvent) => {
			const transfer = e.dataTransfer
			if (!transfer?.types.includes('Files')) { clear(); return }
			e.preventDefault()
			if (!isGraphArchiveDrag(transfer)) { clear(); return }
			const destination = graphDropDestination(e.target, graphSession.backendId)
			const target = e.target instanceof Element ? e.target : null
			const overSidebar = !!target?.closest('.sidebar')
			const root = target?.closest<HTMLDetailsElement>('[data-backend-id]') ?? (overSidebar ? document.querySelector<HTMLDetailsElement>(`#workspace-sidebar [data-backend-id="${CSS.escape(destination.backendId)}"]`) : null)
			if (root && !root.open) root.open = true
			const entry = backendRegistry.entries.find((entry) => entry.id === destination.backendId)
			const folder = destination.groupId ? target?.closest<HTMLElement>('[data-row-kind="dir"]') : null
			const rootBounds = root?.getBoundingClientRect()
			const width = rootBounds ? Math.min(280, rootBounds.width - 16) : 280
			const hintX = rootBounds ? rootBounds.left + 8 : e.clientX + 18
			const hintY = folder ? folder.getBoundingClientRect().bottom + 6 : e.clientY + 18
			const files = [...transfer.files].filter((file) => /\.astelier$/i.test(file.name))
			const allowed = ready && !transferringGraph && entry?.status === 'online'
			transfer.dropEffect = allowed ? 'copy' : 'none'
			archiveHover = {
				...destination,
				backendName: entry?.name ?? '来源不可用',
				folderName: destination.groupId ? folder?.dataset.rowName ?? '文件夹' : '根目录',
				fileLabel: files.length > 1 ? `${files[0].name} 等 ${files.length} 个图包` : files[0]?.name ?? '.astelier 图包',
				allowed, sidebar: overSidebar,
				x: Math.max(12, Math.min(hintX, window.innerWidth - width - 12)),
				y: Math.max(12, Math.min(hintY, window.innerHeight - 156)),
				width,
			}
		}
		const drop = (e: DragEvent) => {
			clear()
			const files = [...(e.dataTransfer?.files ?? [])].filter((file) => /\.astelier$/i.test(file.name))
			if (!files.length) return
			e.preventDefault(); e.stopImmediatePropagation()
			void importGraphs(files, graphDropDestination(e.target, graphSession.backendId))
		}
		const leave = (e: DragEvent) => { if (e.clientX <= 0 || e.clientY <= 0 || e.clientX >= window.innerWidth || e.clientY >= window.innerHeight) clear() }
		const key = (e: KeyboardEvent) => { if (e.key === 'Escape') clear() }
		window.addEventListener('dragover', over, true)
		window.addEventListener('dragenter', over, true)
		window.addEventListener('drop', drop, true)
		window.addEventListener('dragleave', leave, true)
		window.addEventListener('dragend', clear, true)
		window.addEventListener('blur', clear)
		window.addEventListener('keydown', key)
		return () => {
			window.removeEventListener('dragover', over, true); window.removeEventListener('dragenter', over, true); window.removeEventListener('drop', drop, true)
			window.removeEventListener('dragleave', leave, true); window.removeEventListener('dragend', clear, true); window.removeEventListener('blur', clear); window.removeEventListener('keydown', key)
		}
	})

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
			await renameGraphAndSync(activeGraph(), next)
			await sidebar?.refreshAndKeepActive()
		} catch (e) {
			toast({ kind: 'err', title: '重命名失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	onMount(async () => {
		navigator.storage?.persist?.().catch(() => {})
		await connectBackend('local')
		for (const entry of backendRegistry.entries.filter((entry) => entry.kind === 'http')) void connectBackend(entry.id)

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
	<Sidebar bind:this={sidebar} onManage={() => (settingsOpen = true)} collapsed={!sidebarOpen} dropPreview={archiveHover} />
	<div class="main">
		<Topbar title={graphSession.title} saveState={graphSession.saveState} {running} ready={ready && !graphSession.loading} onRename={renameTitle} onRun={run} onManage={() => (settingsOpen = true)} {sidebarOpen} onToggleSidebar={toggleSidebar} onExport={() => void exportGraph()} onImport={chooseGraphImport} transferring={transferringGraph} />
		<div class="canvas-layer" class:archive-target={archiveHover && !archiveHover.sidebar} inert={graphSession.loading}>
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

{#if archiveHover}
	<div class="archive-drop-preview" class:sidebar-preview={archiveHover.sidebar} style:left="{archiveHover.x}px" style:top="{archiveHover.y}px" style:width="{archiveHover.width}px"><GraphDropHint preview={archiveHover} /></div>
{/if}

<input class="ui-file" type="file" accept=".astelier" multiple bind:this={graphImportInput} onchange={(e) => { void importGraphs([...(e.currentTarget.files ?? [])], { ...importDestination }); e.currentTarget.value = '' }} />

<NodePalette bind:this={palette} />
<Toaster />
<ConfirmHost />
<LightboxHost />
<SettingsDialog open={settingsOpen} onclose={() => (settingsOpen = false)} />

<style>
	.canvas-layer.archive-target { outline: 2px dashed var(--ui-accent); outline-offset: -10px; }
	.archive-drop-preview { position: fixed; width: 280px; max-width: calc(100vw - 24px); z-index: 950; pointer-events: none; box-shadow: var(--ui-shadow-float); border-radius: var(--ui-r-menu); }
</style>
