<script lang="ts">
	import { untrack } from 'svelte'
	import { autoFocusSelect } from '../ui/actions/autoFocusSelect'
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import { confirmDialog } from '../ui/confirm/confirm.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { backend, backendStore } from '../backends/registry.svelte'
	let { backendId }: { backendId: string } = $props()
	const store = untrack(() => backendStore(backendId))
	import { type GraphGroup, type GraphSummary } from './types'
	import { graphSession, openGraph, renameGraphAndSync, deleteGraph } from '../canvas/session.svelte'

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

	// 内联新建 / 重命名（draft = 输入框当前值，供 window pointerdown 提交）
	let creating: { kind: 'dir' | 'graph'; parentId: string | null } | null = $state(null)
	let renaming: { kind: 'dir' | 'graph'; id: string } | null = $state(null)
	let renameDraft = $state('')
	let createDraft = $state('')

	// 右键菜单（视口内翻转）
	let menu: { x: number; y: number; target: Row | null } | null = $state(null)

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

	function startCreate(kind: 'dir' | 'graph', parentId: string | null) {
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
				? `删除文件夹「${row.name}」？其中的图将移动到根目录。`
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

	function onContext(e: MouseEvent, row: Row | null) {
		e.preventDefault()
		e.stopPropagation()
		// 视口内翻转，避免菜单超出屏幕
		const x = Math.min(e.clientX, window.innerWidth - 180)
		const y = Math.min(e.clientY, window.innerHeight - 230)
		menu = { x, y, target: row }
	}
	function closeMenu() {
		menu = null
	}
	/** 点击输入框之外的任何地方：提交重命名 / 新建（blur 路径不可靠） */
	function onWinPointerDown(e: PointerEvent) {
		closeMenu()
		const inInput = (e.target as HTMLElement | null)?.closest?.('.rename')
		if (renaming && !inInput) void commitRename(renameDraft)
		else if (creating && !inInput) void commitCreate(createDraft)
	}
	async function menuAct(fn: () => void, row: Row | null) {
		closeMenu()
		if (row?.kind === 'dir') expanded = { ...expanded, [row.id]: true }
		fn()
	}
</script>

<!-- capture 阶段：画布的拖拽层会 stopPropagation 拦掉冒泡的 pointerdown，
     捕获阶段在 window 最先执行，任何地方点击都能收到 -->
<svelte:window
	onpointerdowncapture={onWinPointerDown}
	onpointermove={onWinPointerMove}
	onpointerup={onWinPointerUp}
	onkeydown={(e) => e.key === 'Escape' && closeMenu()}
/>

<div class="backend-graphs" data-graph-backend={backendId} oncontextmenu={(e) => onContext(e, null)} role="navigation">
	<div class="side-head">
		<span class="label">图与分组</span>
		<IconButton
			icon="plus"
			label="新建图（根目录）"
			sm
			onclick={() => startCreate('graph', null)}
		/>
		<IconButton
			icon="folder"
			label="新建文件夹（根目录）"
			sm
			onclick={() => startCreate('dir', null)}
		/>
	</div>

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
			<div class="side-empty">还没有图。<br />点上方 + 或右键新建。</div>
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
			class="ui-menu"
			role="menu"
			tabindex="-1"
			style:left="{menu.x}px"
			style:top="{menu.y}px"
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
				<button type="button" class="ui-menu-item" role="menuitem" onclick={() => menuAct(() => startRename(row), row)}>
					<Icon name="pencil" size={14} />重命名
				</button>
				<button type="button" class="ui-menu-item danger" role="menuitem" onclick={() => menuAct(() => remove(row), row)}>
					<Icon name="trash" size={14} />删除
				</button>
			{:else}
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
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="tree-row"
		data-row-id={row.id}
		data-row-kind={row.kind}
		aria-selected={row.kind === 'graph' && row.id === activeId}
		class:active={row.kind === 'graph' && row.id === activeId}
		class:droppable={dropTarget === row.id && row.kind === 'dir'}
		class:dragging={dragging?.id === row.id}
		style:padding-left="{8 + depth * 14}px"
		role="treeitem"
		tabindex="-1"
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
					expanded = { ...expanded, [row.id]: !expanded[row.id] }
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
			<span class="name">{row.name}</span>
			<span class="row-actions">
				<button
					type="button"
					title="重命名"
					aria-label="重命名"
					onpointerdown={(e) => e.stopPropagation()}
					onclick={(e) => {
						e.stopPropagation()
						startRename(row)
					}}
				>
					<Icon name="pencil" size={12} />
				</button>
				<button
					type="button"
					title="删除"
					aria-label="删除"
					onpointerdown={(e) => e.stopPropagation()}
					onclick={(e) => {
						e.stopPropagation()
						void remove(row)
					}}
				>
					<Icon name="trash" size={12} />
				</button>
			</span>
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
	/* 每棵图树：头部固定，树自己滚动（父级 details 已给它 flex 高度） */
	.backend-graphs {
		display: flex;
		flex-direction: column;
		min-height: 0;
		flex: 1;
	}
	/* 图树头部比侧栏标题矮一档，和节点列表排在一起才不显臃肿 */
	.backend-graphs > :global(.side-head) {
		height: 30px;
		padding: 0 6px 0 10px;
	}
</style>
