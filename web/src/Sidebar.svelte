<script lang="ts">
	import { onMount } from 'svelte'
	import {
		createDir,
		createGraphIn,
		deleteGraph,
		deleteGroup,
		fetchGraphs,
		fetchGroups,
		moveGroup,
		renameGroup,
		setGraphGroup,
		type GraphGroup,
		type GraphSummary,
	} from './api'
	import { activeGraphId, openGraph } from './graphStore'

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

	// 右键菜单
	let menu = $state<{ x: number; y: number; target: Row | null } | null>(null)
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
			.catch((e) => alert(e instanceof Error ? e.message : String(e)))
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
			alert(e instanceof Error ? e.message : String(e))
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
			if (kind === 'dir') await renameGroup(id, name.trim())
			else {
				// 图标题：暂用 PUT group 端点之外的路径 —— 标题随结构保存走服务端补丁
				await renameGraph(id, name.trim())
			}
			await refresh()
		} catch (e) {
			alert(e instanceof Error ? e.message : String(e))
		}
	}

	async function renameGraph(id: string, title: string) {
		const res = await fetch(`/api/graphs/${encodeURIComponent(id)}/title`, {
			method: 'PUT',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ title }),
		})
		if (!res.ok) throw new Error(`重命名失败（${res.status}）`)
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
			if (!confirm(msg)) return
			try {
				await deleteGroup(row.id)
			} catch (e) {
				alert(e instanceof Error ? e.message : String(e))
				return
			}
		} else {
			if (!confirm(`删除图「${row.name}」？此操作不可恢复。`)) return
			try {
				await deleteGraph(row.id)
			} catch (e) {
				alert(e instanceof Error ? e.message : String(e))
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
			alert(e2 instanceof Error ? e2.message : String(e2))
		}
	}

	// ---------- 右键菜单 ----------

	function onContext(e: MouseEvent, row: Row | null) {
		e.preventDefault()
		e.stopPropagation()
		menu = { x: e.clientX, y: e.clientY, target: row }
	}
	function closeMenu() {
		menu = null
	}
	function menuAct(fn: () => void, row: Row | null) {
		closeMenu()
		if (row?.kind === 'dir') expanded = { ...expanded, [row.id]: true }
		fn()
	}
</script>

<svelte:window on:pointerdown={closeMenu} on:keydown={(e) => e.key === 'Escape' && closeMenu()} />

<aside class="sidebar" on:contextmenu={(e) => onContext(e, null)} role="navigation">
	<div class="side-head">
		<span>图库</span>
		<button
			class="mini"
			title="新建图（根目录）"
			on:click={() => startCreate('graph', null)}
			on:pointerdown|stopPropagation
		>+图</button>
		<button
			class="mini"
			title="新建文件夹（根目录）"
			on:click={() => startCreate('dir', null)}
			on:pointerdown|stopPropagation
		>+夹</button>
	</div>

	{#if loadError}
		<div class="err">{loadError}</div>
	{/if}

	<div class="tree" on:dragover={(e) => onDragover(e, 'root')} on:drop={(e) => onDrop(e, null)}>
		{#each tree as row (row.kind + row.id)}
			{@render RowEl(row, 0)}
		{/each}
		{#if creating && creating.parentId === null}
			{@render CreateRow(0)}
		{/if}
		{#if tree.length === 0 && !creating}
			<div class="empty">右键或 + 新建</div>
		{/if}
	</div>

	{#if menu}
		<div class="ctx" style:left="{menu.x}px" style:top="{menu.y}px" on:pointerdown|stopPropagation>
			{#if menu.target?.kind === 'dir'}
				{@const row = menu.target}
				<div class="ctx-item" on:click={() => menuAct(() => startCreate('graph', row.id), row)}>新建图</div>
				<div class="ctx-item" on:click={() => menuAct(() => startCreate('dir', row.id), row)}>新建文件夹</div>
				<div class="ctx-item" on:click={() => menuAct(() => startRename(row), row)}>重命名</div>
				<div class="ctx-item danger" on:click={() => menuAct(() => remove(row), row)}>删除</div>
			{:else if menu.target?.kind === 'graph'}
				{@const row = menu.target}
				<div class="ctx-item" on:click={() => menuAct(() => startRename(row), row)}>重命名</div>
				<div class="ctx-item danger" on:click={() => menuAct(() => remove(row), row)}>删除</div>
			{:else}
				<div class="ctx-item" on:click={() => menuAct(() => startCreate('graph', null), null)}>新建图</div>
				<div class="ctx-item" on:click={() => menuAct(() => startCreate('dir', null), null)}>新建文件夹</div>
			{/if}
		</div>
	{/if}
</aside>

{#snippet RowEl(row: Row, depth: number)}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<div
		class="row"
		class:active={row.kind === 'graph' && row.id === activeId}
		class:droppable={dropTarget === row.id && row.kind === 'dir'}
		style:padding-left="{8 + depth * 14}px"
		draggable="true"
		on:dragstart={(e) => onDragstart(e, row)}
		on:dragover|preventDefault={(e) => row.kind === 'dir' && onDragover(e, row.id)}
		on:dragleave={() => row.kind === 'dir' && dropTarget === row.id && (dropTarget = null)}
		on:drop|stopPropagation={(e) => row.kind === 'dir' && onDrop(e, row.id)}
		on:contextmenu={(e) => onContext(e, row)}
	>
		{#if row.kind === 'dir'}
			<button
				class="twist"
				tabindex="-1"
				on:click|stopPropagation={() => (expanded = { ...expanded, [row.id]: !expanded[row.id] })}
			>
				{expanded[row.id] === false ? '▸' : '▾'}
			</button>
		{:else}
			<span class="twist placeholder"></span>
		{/if}

		{#if renaming && renaming.id === row.id}
			<input
				class="rename"
				value={row.name}
				on:pointerdown|stopPropagation
				on:keydown={(e) => {
					if (e.key === 'Enter') commitRename(e.currentTarget.value)
					else if (e.key === 'Escape') (renaming = null)
				}}
				on:blur={(e) => commitRename(e.currentTarget.value)}
			/>
		{:else}
			<span
				class="name"
				on:click={() => (row.kind === 'dir' ? (expanded = { ...expanded, [row.id]: expanded[row.id] === false }) : open(row))}
				on:dblclick={() => startRename(row)}
			>
				{row.kind === 'dir' ? '📁' : '🖼'} {row.name}
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
	<div class="row" style:padding-left="{8 + depth * 14}px">
		<span class="twist placeholder"></span>
		<input
			class="rename"
			placeholder={creating?.kind === 'dir' ? '文件夹名…' : '图名…'}
			autofocus
			on:pointerdown|stopPropagation
			on:keydown={(e) => {
				if (e.key === 'Enter') commitCreate(e.currentTarget.value)
				else if (e.key === 'Escape') (creating = null)
			}}
			on:blur={(e) => commitCreate(e.currentTarget.value)}
		/>
	</div>
{/snippet}

<style>
	.sidebar {
		width: 220px;
		flex: none;
		display: flex;
		flex-direction: column;
		background: var(--an-panel);
		border-right: 1px solid var(--an-border);
		font-size: 12px;
	}
	.side-head {
		display: flex;
		align-items: center;
		gap: 6px;
		padding: 8px 10px;
		color: var(--an-dim);
		border-bottom: 1px solid var(--an-border);
	}
	.side-head span {
		flex: 1;
		font-weight: 600;
		color: var(--an-text);
	}
	button.mini {
		background: transparent;
		color: var(--an-dim);
		border: 1px solid var(--an-border);
		border-radius: 4px;
		font: inherit;
		font-size: 10px;
		padding: 1px 4px;
		cursor: pointer;
	}
	button.mini:hover {
		color: var(--an-text);
	}
	.tree {
		flex: 1;
		overflow: auto;
		padding: 6px 4px;
		min-height: 0;
	}
	.row {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 3px 6px;
		border-radius: 6px;
		cursor: pointer;
		color: var(--an-text);
	}
	.row:hover {
		background: var(--an-hover);
	}
	.row.active {
		background: var(--an-accent-weak);
	}
	.row.droppable {
		outline: 1px dashed var(--an-accent);
	}
	.twist {
		width: 14px;
		flex: none;
		background: transparent;
		border: none;
		color: var(--an-dim);
		font-size: 9px;
		padding: 0;
		cursor: pointer;
	}
	.twist.placeholder {
		cursor: default;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.rename {
		flex: 1;
		min-width: 0;
		background: var(--an-input);
		border: 1px solid var(--an-accent);
		border-radius: 4px;
		color: var(--an-text);
		font: inherit;
		padding: 2px 4px;
		outline: none;
	}
	.empty,
	.err {
		padding: 10px;
		color: var(--an-dim);
	}
	.err {
		color: var(--an-danger);
	}
	.ctx {
		position: fixed;
		z-index: 200;
		min-width: 132px;
		padding: 4px;
		background: var(--an-panel);
		border: 1px solid var(--an-border);
		border-radius: 8px;
		box-shadow: var(--an-menu-shadow);
		font-size: 12px;
	}
	.ctx-item {
		padding: 5px 10px;
		border-radius: 6px;
		cursor: pointer;
		color: var(--an-text);
	}
	.ctx-item:hover {
		background: var(--an-hover);
	}
	.ctx-item.danger {
		color: var(--an-danger);
	}
</style>
