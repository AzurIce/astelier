<script lang="ts">
	import { tick, untrack } from 'svelte'
	import { autoFocusSelect } from '../ui/actions/autoFocusSelect'
	import Icon from '../ui/Icon.svelte'
	import { confirmDialog } from '../ui/confirm/confirm.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { backend, backendStore } from '../backends/registry.svelte'
	let { backendId, dropPreview = null }: { backendId: string; dropPreview?: GraphDropPreview | null } = $props()
	const store = untrack(() => backendStore(backendId))
	import { type GraphGroup, type GraphSummary } from './types'
	import { graphSession, openGraph, renameGraphAndSync, deleteGraph, flushNow } from '../canvas/session.svelte'
	import { createGraphArchive } from './graphArchive'
	import { downloadFile } from '../ui/download'
	import type { GraphDropPreview } from './graphDrop'

	interface Row {
		kind: 'dir' | 'graph'
		id: string
		name: string
		parentId: string | null
		children: Row[]
	}

	let dirs: GraphGroup[] = $state([])
	let graphs: GraphSummary[] = $state([])
	let activeId = $derived(graphSession.backendId === backendId ? graphSession.id : '')
	let expanded: Record<string, boolean> = $state({})
	let loadError: string | null = $state(null)
	let exporting = $state(false)
	async function exportGraph(row: Row) {
		if (exporting) return
		exporting = true
		try { await flushNow(); downloadFile(await createGraphArchive(store, row.id)) }
		catch (error) { toast({ kind: 'err', title: '导出图失败', msg: String(error) }) }
		finally { exporting = false }
	}

	// 内联新建 / 重命名（draft = 输入框当前值，供 window pointerdown 提交）
	let creating: { kind: 'dir' | 'graph'; parentId: string | null } | null = $state(null)
	let renaming: { kind: 'dir' | 'graph'; id: string } | null = $state(null)
	let renameDraft = $state('')
	let createDraft = $state('')

	// 右键菜单（视口内翻转）
	let menu: { x: number; y: number; target: Row | null } | null = $state(null)
	let menuElement: HTMLDivElement | undefined = $state()
	let menuTrigger: HTMLElement | null = null

	$effect(() => {
		const current = menu
		if (!current || !menuElement) return
		void tick().then(() => {
			if (menu !== current || !menuElement) return
			const rect = menuElement.getBoundingClientRect()
			menuElement.style.left = `${Math.max(8, Math.min(current.x, window.innerWidth - rect.width - 8))}px`
			menuElement.style.top = `${Math.max(8, Math.min(current.y, window.innerHeight - rect.height - 8))}px`
			menuElement.querySelector<HTMLButtonElement>('button')?.focus()
		})
	})

	// 拖拽移动
	let dragging: { kind: 'dir' | 'graph'; id: string } | null = $state(null)
	let dropTarget: string | null = $state(null) // 'root' 或目录 id

	let tree = $derived(buildTree(dirs, graphs))

	function buildTree(d: GraphGroup[], g: GraphSummary[]): Row[] {
		const byParent = new Map<string | null, GraphGroup[]>()
		for (const dir of [...d].sort((a, b) => a.name.localeCompare(b.name))) {
			const key = dir.parent_id
			if (!byParent.has(key)) byParent.set(key, [])
			byParent.get(key)!.push(dir)
		}
		const byGroup = new Map<string | null, GraphSummary[]>()
		for (const graph of [...g].sort((a, b) => a.title.localeCompare(b.title))) {
			const key = graph.group_id
			if (!byGroup.has(key)) byGroup.set(key, [])
			byGroup.get(key)!.push(graph)
		}
		function assemble(parentId: string | null): Row[] {
			const rows: Row[] = []
			for (const dir of byParent.get(parentId) ?? []) {
				rows.push({
					kind: 'dir',
					id: dir.id,
					name: dir.name,
					parentId: dir.parent_id,
					children: assemble(dir.id),
				})
			}
			for (const graph of byGroup.get(parentId) ?? []) {
				rows.push({ kind: 'graph', id: graph.id, name: graph.title, parentId: graph.group_id, children: [] })
			}
			return rows
		}
		return assemble(null)
	}

	async function refresh() {
		try {
			;[dirs, graphs] = await Promise.all([store.listGroups(), store.listGraphs()])
			loadError = null
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e)
		}
	}

	/** 顶层 App 在建图 / 删图后调用，保持列表同步 */
	export async function refreshAndKeepActive() {
		await refresh()
	}

	$effect(() => { backend(backendId).revision; untrack(() => void refresh()) })

	function open(row: Row) {
		if (row.kind !== 'graph') return
		openGraph({ backendId, id: row.id })
			.catch((e) => toast({ kind: 'err', title: '打开失败', msg: String(e) }))
	}

	// ---------- 新建 / 重命名 ----------

	export function startCreate(kind: 'dir' | 'graph', parentId: string | null = null) {
		expanded = { ...expanded, [parentId ?? 'root']: true }
		creating = { kind, parentId }
		renaming = null
		createDraft = ''
	}

	async function commitCreate(name: string) {
		if (!creating) return
		const { kind, parentId } = creating
		creating = null
		if (!name.trim()) return
		try {
			if (kind === 'dir') {
				const dir = await store.createGroup(name.trim(), parentId)
				expanded = { ...expanded, [dir.id]: true }
			} else {
				const doc = await store.createGraph(parentId, name.trim())
				await refresh()
				await openGraph({ backendId, id: doc.id })
			}
			await refresh()
		} catch (e) {
			toast({ kind: 'err', title: '新建失败', msg: String(e) })
		}
	}

	function startRename(row: Row) {
		creating = null
		renaming = { kind: row.kind, id: row.id }
		renameDraft = row.name
	}

	async function commitRename(name: string) {
		if (!renaming) return
		const { kind, id } = renaming
		renaming = null
		if (!name.trim()) return
		try {
			if (kind === 'dir') {
				await store.renameGroup(id, name.trim())
			} else {
				// 标题更新同步活动会话，图身份保持稳定
				await renameGraphAndSync({ backendId, id }, name.trim())
			}
			await refresh()
		} catch (e) {
			toast({ kind: 'err', title: '重命名失败', msg: String(e) })
		}
	}

	// ---------- 删除 ----------

	async function remove(row: Row) {
		if (row.kind === 'dir') {
			const hasContent =
				row.children.length > 0 ||
				dirs.some((d) => d.parent_id === row.id) ||
				graphs.some((g) => g.group_id === row.id)
			const msg = hasContent
				? `删除文件夹「${row.name}」及其子文件夹？其中的图将保留并移动到根目录。`
				: `删除空文件夹「${row.name}」？`
			if (!(await confirmDialog({ title: '删除文件夹', message: msg, confirmText: '删除', danger: true })))
				return
			try {
				await store.deleteGroup(row.id)
			} catch (e) {
				toast({ kind: 'err', title: '删除失败', msg: String(e) })
				return
			}
		} else {
			if (
				!(await confirmDialog({
					title: '删除图',
					message: `删除「${row.name}」？此操作不可恢复。`,
					confirmText: '删除',
					danger: true,
				}))
			)
				return
			try {
				await deleteGraph({ backendId, id: row.id })
			} catch (e) {
				toast({ kind: 'err', title: '删除失败', msg: String(e) })
				return
			}
		}
		await refresh()
	}

	// ---------- 拖拽移动（pointer 事件） ----------
	// 5px 阈值区分「点击」（打开图 / 选中目录）与「拖动」；拖动目标由
	// elementFromPoint 命中 .tree-row 决定：目录行 → 移入该目录；侧栏
	// 空白（含画布方向外）→ 根目录；图行 / 拖出侧栏无效目标 → 松手取消。

	let pressStart: { x: number; y: number; row: Row } | null = null
	let dragMoved = false

	function onRowPointerDown(e: PointerEvent, row: Row) {
		if (e.button !== 0) return
		if ((e.target as HTMLElement | null)?.closest?.('button, input, textarea')) return
		pressStart = { x: e.clientX, y: e.clientY, row }
		dragMoved = false
	}

	function onWinPointerMove(e: PointerEvent) {
		if (!pressStart) return
		const dx = e.clientX - pressStart.x
		const dy = e.clientY - pressStart.y
		if (!dragMoved) {
			if (Math.hypot(dx, dy) < 5) return
			dragMoved = true
			dragging = { kind: pressStart.row.kind, id: pressStart.row.id }
		}
		const el = document.elementFromPoint(e.clientX, e.clientY)
		const rowEl = el?.closest<HTMLElement>('.tree-row')
		const inSidebar = rowEl?.closest<HTMLElement>('[data-graph-backend]')?.dataset.graphBackend === backendId
		if (inSidebar && rowEl?.dataset.rowKind === 'dir' && rowEl.dataset.rowId !== dragging?.id) {
			dropTarget = rowEl.dataset.rowId!
		} else if (inSidebar) {
			dropTarget = 'root'
		} else {
			dropTarget = null // 无效目标：松手取消
		}
	}

	function onWinPointerUp() {
		const d = dragging
		const target = dropTarget
		pressStart = null
		dragMoved = false
		dragging = null
		dropTarget = null
		if (!d) return
		if (target === null) return // 取消
		void (async () => {
			try {
				if (d.kind === 'graph') await store.setGraphGroup(d.id, target === 'root' ? null : target)
				else await store.moveGroup(d.id, target === 'root' ? null : target)
				await refresh()
			} catch (e2) {
				toast({ kind: 'err', title: '移动失败', msg: String(e2) })
			}
		})()
	}


	// ---------- 右键菜单 ----------

	export function openRootMenu(e: MouseEvent) { onContext(e, null) }

	function onContext(e: MouseEvent, row: Row | null) {
		e.preventDefault()
		e.stopPropagation()
		menuTrigger = e.currentTarget as HTMLElement
		menu = { x: e.clientX, y: e.clientY, target: row }
	}
	function showRowMenu(e: MouseEvent, row: Row) {
		e.stopPropagation()
		const trigger = e.currentTarget as HTMLElement
		const rect = trigger.getBoundingClientRect()
		menuTrigger = trigger
		menu = { x: rect.left, y: rect.bottom + 4, target: row }
	}
	export function closeMenu(restoreFocus = false) {
		if (!menu) return
		menu = null
		if (restoreFocus) menuTrigger?.focus()
	}
	function onMenuKeydown(e: KeyboardEvent) {
		const buttons = [...(menuElement?.querySelectorAll<HTMLButtonElement>('button') ?? [])]
		const index = buttons.indexOf(document.activeElement as HTMLButtonElement)
		if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
			e.preventDefault()
			const next = e.key === 'Home' ? 0 : e.key === 'End' ? buttons.length - 1 : (index + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length
			buttons[next]?.focus()
		} else if (e.key === 'Tab') closeMenu()
	}
	/** 点击输入框之外的任何地方：提交重命名 / 新建（blur 路径不可靠） */
	function onWinPointerDown(e: PointerEvent) {
		// 捕获阶段不能卸载菜单内的按钮，否则后续 click 无法执行。
		if (menuElement?.contains(e.target as Node)) return
		closeMenu()
		const inInput = (e.target as HTMLElement | null)?.closest?.('.rename')
		if (renaming && !inInput) void commitRename(renameDraft)
		else if (creating && !inInput) void commitCreate(createDraft)
	}
	function menuAct(fn: () => void, row: Row | null) {
		if (row?.kind === 'dir') expanded = { ...expanded, [row.id]: true }
		// 菜单模板的行引用依赖 menu，先将目标传给操作，再卸载菜单。
		fn()
		closeMenu()
	}
</script>

<!-- capture 阶段：画布的拖拽层会 stopPropagation 拦掉冒泡的 pointerdown，
     捕获阶段在 window 最先执行，任何地方点击都能收到 -->
<svelte:window
	onpointerdowncapture={onWinPointerDown}
	onpointermove={onWinPointerMove}
	onpointerup={onWinPointerUp}
	onkeydown={(e) => e.key === 'Escape' && closeMenu(true)}
	onresize={() => closeMenu()}
/>

<div class="backend-graphs" data-graph-backend={backendId} oncontextmenu={openRootMenu} role="navigation">
	{#if loadError}
		<div class="side-err">{loadError}</div>
	{/if}

	<div class="tree" role="tree" tabindex="-1">
		{#each tree as row (row.kind + row.id)}
			{@render RowEl(row, 0)}
		{/each}
		{#if creating && creating.parentId === null}
			{@render CreateRow(0)}
		{/if}
		{#if tree.length === 0 && !creating}
			<div class="side-empty">还没有图，点上方 + 新建。</div>
		{/if}
	</div>

	{#if dragging}
		{#if dropTarget === null}
			<div class="drop-hint muted">松手取消移动</div>
		{:else}
			{@const targetName =
				dropTarget === 'root'
					? '根目录'
					: (dirs.find((d) => d.id === dropTarget)?.name ?? '…')}
			<div class="drop-hint">
				<Icon name="folder" size={13} />
				<span>松手移动到「{targetName}」</span>
			</div>
		{/if}
	{/if}

	{#if menu}
		<div
			bind:this={menuElement}
			class="ui-menu"
			role="menu"
			tabindex="-1"
			style:left="{menu.x}px"
			style:top="{menu.y}px"
			onkeydown={onMenuKeydown}
			oncontextmenu={(e) => { e.preventDefault(); e.stopPropagation() }}
		>
			{#if menu.target?.kind === 'dir'}
				{@const row = menu.target}
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startCreate('graph', row.id), row)}>
					<Icon name="plus" size={14} />新建图
				</button>
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startCreate('dir', row.id), row)}>
					<Icon name="folder" size={14} />新建文件夹
				</button>
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startRename(row), row)}>
					<Icon name="pencil" size={14} />重命名
				</button>
				<button type="button" class="ui-menu-item danger" role="menuitem" onclick={() => menuAct(() => remove(row), row)}>
					<Icon name="trash" size={14} />删除
				</button>
			{:else if menu.target?.kind === 'graph'}
				{@const row = menu.target}
				<button type="button" class="ui-menu-item" role="menuitem" disabled={exporting} onclick={() => menuAct(() => void exportGraph(row), row)}><Icon name="upload" size={14} />导出 .astelier</button>
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startRename(row), row)}>
					<Icon name="pencil" size={14} />重命名
				</button>
				<button type="button" class="ui-menu-item danger" role="menuitem" onclick={() => menuAct(() => remove(row), row)}>
					<Icon name="trash" size={14} />删除
				</button>
			{:else}
				<div class="ui-menu-label">{backend(backendId).name}</div>
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startCreate('graph', null), null)}>
					<Icon name="plus" size={14} />新建图
				</button>
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startCreate('dir', null), null)}>
					<Icon name="folder" size={14} />新建文件夹
				</button>
			{/if}
		</div>
	{/if}
</div>

{#snippet RowEl(row: Row, depth: number)}
	<div
		class="tree-row"
		data-row-id={row.id}
		data-row-kind={row.kind}
		data-row-name={row.name}
		aria-selected={row.kind === 'graph' && row.id === activeId}
		aria-expanded={row.kind === 'dir' ? expanded[row.id] !== false : undefined}
		class:active={row.kind === 'graph' && row.id === activeId}
		class:droppable={dropTarget === row.id && row.kind === 'dir'}
		class:dragging={dragging?.id === row.id}
		class:archive-folder={dropPreview?.groupId === row.id && row.kind === 'dir'}
		style:padding-left="{8 + depth * 14}px"
		role="treeitem"
		tabindex="0"
		onkeydown={(e) => {
			if (e.target !== e.currentTarget) return
			if (e.key === 'Enter' || e.key === ' ') {
				e.preventDefault()
				if (row.kind === 'dir') expanded = { ...expanded, [row.id]: expanded[row.id] === false }
				else open(row)
			} else if (e.key === 'F2') { e.preventDefault(); startRename(row) }
		}}
		onpointerdown={(e) => onRowPointerDown(e, row)}
		oncontextmenu={(e) => onContext(e, row)}
		onclick={() => {
			// 重命名输入框的点击会冒泡到行上，此时不做行展开/打开
			if (renaming?.id === row.id) return
			if (row.kind === 'dir') expanded = { ...expanded, [row.id]: expanded[row.id] === false }
			else open(row)
		}}
		ondblclick={() => startRename(row)}
	>
		{#if row.kind === 'dir'}
			<button
				type="button"
				class="twisty"
				class:collapsed={expanded[row.id] === false}
				tabindex="-1"
				aria-label="展开/折叠"
				onclick={(e) => {
					e.stopPropagation()
					expanded = { ...expanded, [row.id]: expanded[row.id] === false }
				}}
			>
				<Icon name="chevronDown" size={12} />
			</button>
		{:else}
			<span class="twisty placeholder"></span>
		{/if}

		<span class="tree-icon">
			<Icon name={row.kind === 'dir' ? 'folder' : 'graph'} size={14} />
		</span>

		{#if renaming && renaming.id === row.id}
			<input
				class="rename"
				value={row.name}
				use:autoFocusSelect
				onpointerdown={(e) => e.stopPropagation()}
				oninput={(e) => (renameDraft = (e.target as HTMLInputElement).value)}
				onkeydown={(e) => {
					if (e.key === 'Enter') void commitRename((e.target as HTMLInputElement).value)
					else if (e.key === 'Escape') renaming = null
				}}
				onblur={(e) => void commitRename((e.target as HTMLInputElement).value)}
			/>
		{:else}
			<span class="name" title={row.name}>{row.name}</span>
			<button
				type="button"
				class="row-menu"
				title="更多操作：新建、重命名、删除"
				aria-label="更多操作：{row.name}"
				aria-haspopup="menu"
				aria-expanded={menu?.target?.id === row.id && menu?.target?.kind === row.kind}
				onclick={(e) => showRowMenu(e, row)}
				ondblclick={(e) => e.stopPropagation()}
			>
				<Icon name="more" size={14} />
			</button>
		{/if}
	</div>

	{#if row.kind === 'dir' && expanded[row.id] !== false}
		{#each row.children as child (child.kind + child.id)}
			{@render RowEl(child, depth + 1)}
		{/each}
		{#if creating && creating.parentId === row.id}
			{@render CreateRow(depth + 1)}
		{/if}
	{/if}
{/snippet}

{#snippet CreateRow(depth: number)}
	<div class="tree-row" style:padding-left="{8 + depth * 14}px">
		<span class="twisty placeholder"></span>
		<span class="tree-icon">
			<Icon name={creating?.kind === 'dir' ? 'folder' : 'graph'} size={14} />
		</span>
		<input
			class="rename"
			placeholder={creating?.kind === 'dir' ? '文件夹名…' : '图名…'}
			use:autoFocusSelect
			onpointerdown={(e) => e.stopPropagation()}
			onclick={(e) => e.stopPropagation()}
			oninput={(e) => (createDraft = (e.target as HTMLInputElement).value)}
			onkeydown={(e) => {
				if (e.key === 'Enter') void commitCreate((e.target as HTMLInputElement).value)
				else if (e.key === 'Escape') creating = null
			}}
			onblur={(e) => void commitCreate((e.target as HTMLInputElement).value)}
		/>
	</div>
{/snippet}

<style>
	.backend-graphs {
		position: relative;
		display: flex;
		flex-direction: column;
	}
	.tree { flex: none; overflow: visible; padding-bottom: 28px; }
	.tree-row.archive-folder { background: var(--ui-accent-weak); outline: 1px solid var(--ui-accent); outline-offset: -1px; }
	.row-menu {
		display: grid;
		place-items: center;
		flex: none;
		width: 24px;
		height: 24px;
		border: none;
		border-radius: var(--ui-r-control);
		background: transparent;
		color: var(--ui-faint);
		cursor: pointer;
	}
	.row-menu:hover, .row-menu[aria-expanded='true'] { color: var(--ui-text); background: var(--ui-track); }
	.row-menu:focus-visible, .tree-row:focus-visible { outline: 1px solid var(--ui-accent); outline-offset: -1px; }
</style>
