<script lang="ts">
	import { onMount } from 'svelte'
	import Icon from './components/Icon.svelte'
	import IconButton from './components/IconButton.svelte'
	import { confirmDialog } from './components/confirm.svelte'
	import { toast } from './components/toast.svelte'
	import {
		createDir,
		createGraphIn,
		deleteGraph,
		deleteGroup,
		fetchGraphs,
		fetchGroups,
		moveGroup,
		renameGraph,
		renameGroup,
		setGraphGroup,
		type GraphGroup,
		type GraphSummary,
	} from './api'
	import { activeGraphId, openGraph, setActiveGraphId } from './graphStore'

	interface Row {
		kind: 'dir' | 'graph'
		id: string
		name: string
		parentId: string | null
		children: Row[]
	}

	let dirs: GraphGroup[] = $state([])
	let graphs: GraphSummary[] = $state([])
	let activeId = $state('')
	let expanded: Record<string, boolean> = $state({})
	let loadError: string | null = $state(null)

	// 内联新建 / 重命名
	let creating: { kind: 'dir' | 'graph'; parentId: string | null } | null = $state(null)
	let renaming: { kind: 'dir' | 'graph'; id: string } | null = $state(null)

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
			;[dirs, graphs] = await Promise.all([fetchGroups(), fetchGraphs()])
			activeId = activeGraphId()
			loadError = null
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e)
		}
	}

	/** 顶层 App 在建图 / 删图后调用，保持列表同步 */
	export async function refreshAndKeepActive() {
		await refresh()
	}

	onMount(refresh)

	function open(row: Row) {
		if (row.kind !== 'graph') return
		openGraph(row.id)
			.then(() => {
				activeId = row.id
			})
			.catch((e) => toast({ kind: 'err', title: '打开失败', msg: String(e) }))
	}

	// ---------- 新建 / 重命名 ----------

	function startCreate(kind: 'dir' | 'graph', parentId: string | null) {
		expanded = { ...expanded, [parentId ?? 'root']: true }
		creating = { kind, parentId }
		renaming = null
	}

	async function commitCreate(name: string) {
		if (!creating) return
		const { kind, parentId } = creating
		creating = null
		if (!name.trim()) return
		try {
			if (kind === 'dir') {
				const dir = await createDir(name.trim(), parentId)
				expanded = { ...expanded, [dir.id]: true }
			} else {
				const doc = await createGraphIn(parentId, name.trim())
				await refresh()
				await openGraph(doc.id)
			}
			await refresh()
		} catch (e) {
			toast({ kind: 'err', title: '新建失败', msg: String(e) })
		}
	}

	function startRename(row: Row) {
		creating = null
		renaming = { kind: row.kind, id: row.id }
	}

	async function commitRename(name: string) {
		if (!renaming) return
		const { kind, id } = renaming
		renaming = null
		if (!name.trim()) return
		try {
			if (kind === 'dir') {
				await renameGroup(id, name.trim())
			} else {
				// 重命名会迁移图目录（目录名=图名），返回新 id；
				// 改的是当前打开的图时同步本地活动记录
				const { id: newId } = await renameGraph(id, name.trim())
				if (id === activeId) setActiveGraphId(newId)
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
				await deleteGroup(row.id)
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
				await deleteGraph(row.id)
			} catch (e) {
				toast({ kind: 'err', title: '删除失败', msg: String(e) })
				return
			}
			if (row.id === activeId) {
				// 活动图被删：本地记录清掉，ensure 会新建种子图
				localStorage.removeItem('atelier-graph-id')
				location.reload()
				return
			}
		}
		await refresh()
	}

	// ---------- 拖拽移动 ----------

	function onDragstart(e: DragEvent, row: Row) {
		dragging = { kind: row.kind, id: row.id }
		e.dataTransfer?.setData('text/plain', row.id)
	}
	function onDragover(e: DragEvent, dirId: string | null) {
		if (!dragging) return
		if (dragging.id === dirId) return
		e.preventDefault()
		dropTarget = dirId
	}
	async function onDrop(e: DragEvent, parentId: string | null) {
		e.preventDefault()
		const d = dragging
		dragging = null
		dropTarget = null
		if (!d) return
		try {
			if (d.kind === 'graph') await setGraphGroup(d.id, parentId)
			else await moveGroup(d.id, parentId)
			await refresh()
		} catch (e2) {
			toast({ kind: 'err', title: '移动失败', msg: String(e2) })
		}
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
	async function menuAct(fn: () => void, row: Row | null) {
		closeMenu()
		if (row?.kind === 'dir') expanded = { ...expanded, [row.id]: true }
		fn()
	}
</script>

<svelte:window onpointerdown={closeMenu} onkeydown={(e) => e.key === 'Escape' && closeMenu()} />

<aside class="sidebar" role="navigation" oncontextmenu={(e) => onContext(e, null)}>
	<div class="side-head">
		<span class="label">图库</span>
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

	<div
		class="tree"
		role="tree"
		tabindex="-1"
		ondragover={(e) => onDragover(e, 'root')}
		ondrop={(e) => onDrop(e, null)}
	>
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
</aside>

{#snippet RowEl(row: Row, depth: number)}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="tree-row"
		aria-selected={row.kind === 'graph' && row.id === activeId}
		class:active={row.kind === 'graph' && row.id === activeId}
		class:droppable={dropTarget === row.id && row.kind === 'dir'}
		style:padding-left="{8 + depth * 14}px"
		role="treeitem"
		tabindex="-1"
		draggable="true"
		ondragstart={(e) => onDragstart(e, row)}
		ondragover={(e) => {
			if (row.kind === 'dir') onDragover(e, row.id)
		}}
		ondragleave={() => {
			if (row.kind === 'dir' && dropTarget === row.id) dropTarget = null
		}}
		ondrop={(e) => {
			if (row.kind === 'dir') onDrop(e, row.id)
		}}
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
				onpointerdown={(e) => e.stopPropagation()}
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
		<!-- svelte-ignore a11y_autofocus -->
		<input
			class="rename"
			placeholder={creating?.kind === 'dir' ? '文件夹名…' : '图名…'}
			autofocus
			onpointerdown={(e) => e.stopPropagation()}
			onclick={(e) => e.stopPropagation()}
			onkeydown={(e) => {
				if (e.key === 'Enter') void commitCreate((e.target as HTMLInputElement).value)
				else if (e.key === 'Escape') creating = null
			}}
			onblur={(e) => void commitCreate((e.target as HTMLInputElement).value)}
		/>
	</div>
{/snippet}
