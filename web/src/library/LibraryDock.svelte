<script lang="ts">
	import { onMount, onDestroy, tick, type Snippet } from 'svelte'
	import Icon from '../ui/Icon.svelte'
	import IconButton from '../ui/IconButton.svelte'
	import { toast } from '../ui/toast/toast.svelte'
	import { openLightbox } from '../ui/lightbox/lightbox.svelte'
	import { STORE_DRAG_MIME, writeImageDrag } from '../images/drag'
	import { baseName, joinPath, parentDir } from './paths'
	import {
		deleteStorePath,
		fetchStoreTree,
		makeStoreDir,
		moveStorePath,
		storeUrl,
		uploadStoreFile,
		type StoreFileEntry,
		type StoreTree,
	} from './api'

	// 底部「库」内容浏览器（对标 UE Content Browser）：
	// 侧边目录树 + 缩略图网格 + 缩放 + 单选/多选/框选 + 右键菜单 + 拖拽移动。
	// 全局库 data/stores/ 是层级命名空间；路径一律用相对路径（'/' 分隔）。
	let {
		collapsed = false,
		ontoggle,
		children,
	}: { collapsed?: boolean; ontoggle?: () => void; children?: Snippet } = $props()

	// ---------- 状态 ----------
	let tree = $state<StoreTree | null>(null)
	let loading = $state(false)
	let cwd = $state('') // 当前目录（'' = 根）
	let expanded = $state(new Set<string>())
	let selected = $state(new Set<string>())
	let tile = $state(128) // 缩略图边长 px
	let query = $state('')
	let maximized = $state(false)
	let panelH = $state(320)
	let renaming = $state<string | null>(null)
	let renameValue = $state('')
	let renameEl = $state<HTMLInputElement>()
	let hovered = $state(false)
	let marquee = $state<{ x: number; y: number; x1: number; y1: number } | null>(null)
	let dropTarget = $state<string | null>(null) // 拖拽悬停的目录（'' = 根，null = 无）
	let ctx = $state<{ x: number; y: number; kind: 'tile' | 'blank'; path: string } | null>(null)
	let uploadRef = $state<HTMLInputElement>()
	let gridEl = $state<HTMLDivElement>()

	// ---------- 数据 ----------
	async function refresh() {
		loading = true
		try {
			tree = await fetchStoreTree()
		} catch (e) {
			toast({ kind: 'err', title: '读取库失败', msg: e instanceof Error ? e.message : String(e) })
		}
		loading = false
	}
	onMount(refresh)

	/** 树结构：path → 直接子目录 / 直接子文件 */
	let dirsByParent = $derived.by(() => {
		const m = new Map<string, string[]>()
		for (const d of tree?.dirs ?? []) {
			const p = parentDir(d)
			if (!m.has(p)) m.set(p, [])
			m.get(p)!.push(d)
		}
		return m
	})
	let filesByParent = $derived.by(() => {
		const m = new Map<string, StoreFileEntry[]>()
		for (const f of tree?.files ?? []) {
			const p = parentDir(f.path)
			if (!m.has(p)) m.set(p, [])
			m.get(p)!.push(f)
		}
		return m
	})
	function fileOf(path: string): StoreFileEntry | undefined {
		return (tree?.files ?? []).find((f) => f.path === path)
	}
	function childDirCount(dir: string): number {
		return (filesByParent.get(dir) ?? []).length
	}

	/** 当前目录的直接子项：目录在前、按名排序；query 过滤 */
	let entries = $derived.by(() => {
		const q = query.trim().toLowerCase()
		const dirs = (dirsByParent.get(cwd) ?? []).filter((d) => !q || baseName(d).toLowerCase().includes(q))
		const files = (filesByParent.get(cwd) ?? []).filter((f) => !q || baseName(f.path).toLowerCase().includes(q))
		dirs.sort((a, b) => baseName(a).localeCompare(baseName(b), 'zh'))
		files.sort((a, b) => baseName(a.path).localeCompare(baseName(b.path), 'zh'))
		return [
			...dirs.map((d) => ({ kind: 'dir' as const, path: d })),
			...files.map((f) => ({ kind: 'file' as const, path: f.path, f })),
		]
	})

	let breadcrumb = $derived(cwd ? cwd.split('/') : [])
	let selCount = $derived(selected.size)
	let selBytes = $derived(
		[...selected].reduce((n, p) => n + (fileOf(p)?.bytes ?? 0), 0),
	)
	let totalHere = $derived(entries.length)

	function fmtSize(n: number): string {
		if (n < 1024) return `${n} B`
		if (n < 1024 * 1024) return `${(n / 1024).toFixed(0)} KB`
		return `${(n / 1048576).toFixed(1)} MB`
	}

	// ---------- 选择 ----------
	function setSel(next: Set<string>) {
		selected = next
	}
	function selectOnly(p: string) {
		setSel(new Set([p]))
	}
	function toggleSel(p: string) {
		const s = new Set(selected)
		if (s.has(p)) s.delete(p)
		else s.add(p)
		setSel(s)
	}
	function selectRange(toPath: string) {
		const i = entries.findIndex((e) => e.path === toPath)
		if (i < 0) return
		const anchor = [...selected][0] ?? toPath
		const a = entries.findIndex((e) => e.path === anchor)
		const [lo, hi] = a <= i ? [a, i] : [i, a]
		const s = new Set<string>()
		for (let k = lo; k <= hi; k++) s.add(entries[k].path)
		setSel(s)
	}
	function selectAll() {
		setSel(new Set(entries.map((e) => e.path)))
	}
	function clearSel() {
		setSel(new Set())
	}

	// ---------- 导航 ----------
	function navigate(dir: string) {
		cwd = dir
		clearSel()
		// 展开祖先链，树上看得到当前位置
		const s = new Set(expanded)
		let p = dir
		while (p) {
			s.add(parentDir(p) || '__root__')
			p = parentDir(p)
		}
		s.delete('__root__')
		expanded = s
		ctx = null
	}
	function isExpanded(dir: string): boolean {
		return expanded.has(dir)
	}
	function toggleExpand(dir: string) {
		const s = new Set(expanded)
		if (s.has(dir)) s.delete(dir)
		else s.add(dir)
		expanded = s
	}

	/** 树的行：根 + 递归展开的目录（flat 列表渲染） */
	let treeRows = $derived.by(() => {
		const rows: { path: string; name: string; depth: number }[] = [{ path: '', name: '库', depth: 0 }]
		const walk = (prefix: string, depth: number) => {
			const kids = [...(dirsByParent.get(prefix) ?? [])].sort((a, b) =>
				baseName(a).localeCompare(baseName(b), 'zh'),
			)
			for (const k of kids) {
				rows.push({ path: k, name: baseName(k), depth })
				if (expanded.has(k)) walk(k, depth + 1)
			}
		}
		walk('', 1)
		return rows
	})

	// ---------- 操作 ----------
	async function upload(list: FileList | null) {
		if (!list?.length) return
		let ok = 0
		for (const f of list) {
			try {
				await uploadStoreFile(f, cwd)
				ok++
			} catch (e) {
				toast({ kind: 'err', title: `上传 ${f.name} 失败`, msg: e instanceof Error ? e.message : String(e) })
			}
		}
		if (ok) toast({ kind: 'ok', title: `已上传 ${ok} 张`, msg: cwd || '库根目录' })
		await refresh()
	}

	/** 结果图 / 外部 URL 拖进来 → 收藏进当前目录 */
	async function collect(url: string) {
		try {
			const res = await fetch(url)
			if (!res.ok) throw new Error(`HTTP ${res.status}`)
			const blob = await res.blob()
			const name = /^(data:|blob:)/i.test(url)
				? `image-${Date.now()}.${blob.type === 'image/jpeg' ? 'jpg' : blob.type === 'image/webp' ? 'webp' : 'png'}`
				: decodeURIComponent(url.split('/').pop() || `image-${Date.now()}.png`)
			await uploadStoreFile(new File([blob], name, { type: blob.type || 'image/png' }), cwd)
			toast({ kind: 'ok', title: '已收入库', msg: `${name} → ${cwd || '根目录'}` })
			await refresh()
		} catch (e) {
			toast({ kind: 'err', title: '收藏失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	async function remove(paths: string[]) {
		if (!paths.length) return
		let ok = 0
		for (const p of paths) {
			try {
				await deleteStorePath(p)
				ok++
			} catch (e) {
				toast({ kind: 'err', title: `删除 ${baseName(p)} 失败`, msg: e instanceof Error ? e.message : String(e) })
			}
		}
		if (ok) {
			toast({ kind: 'ok', title: `已删除 ${ok} 项` })
			await refresh()
			clearSel()
		}
	}

	async function moveInto(paths: string[], dir: string) {
		const targets = paths.filter((p) => p !== dir && parentDir(p) !== dir)
		if (!targets.length) return
		let ok = 0
		let lastErr = ''
		for (const p of targets) {
			try {
				await moveStorePath(p, joinPath(dir, baseName(p)))
				ok++
			} catch (e) {
				lastErr = e instanceof Error ? e.message : String(e)
			}
		}
		if (ok) {
			toast({ kind: 'ok', title: `已移动 ${ok} 项`, msg: `→ ${dir || '库根目录'}` })
			await refresh()
			clearSel()
		}
		if (lastErr && !ok) toast({ kind: 'err', title: '移动失败', msg: lastErr })
	}

	async function startRename(path: string) {
		renaming = path
		renameValue = baseName(path)
		await tick()
		renameEl?.focus()
		// 只选中主体（不含扩展名）：防止误改扩展名导致文件「消失」
		// （扩展名是类型标识，改了会破坏引用与静态 mime；目录无扩展名则全选）
		const dot = renameValue.lastIndexOf('.')
		if (dot > 0) renameEl?.setSelectionRange(0, dot)
		else renameEl?.select()
	}
	async function commitRename() {
		const path = renaming
		renaming = null
		if (!path) return
		const orig = baseName(path)
		const typed = renameValue.trim()
		// 扩展名锁定：输入只改主体，扩展名一律保留原值
		const origDot = orig.lastIndexOf('.')
		const ext = origDot > 0 ? orig.slice(origDot) : ''
		const typedDot = typed.lastIndexOf('.')
		const final = (typedDot > 0 ? typed.slice(0, typedDot) : typed) + ext
		if (!final || final === orig) return
		const to = joinPath(parentDir(path), final)
		try {
			await moveStorePath(path, to)
			await refresh()
			setSel(new Set([to]))
			if (typed !== final) {
				toast({ kind: 'info', title: '扩展名不可修改', msg: `已保留 ${ext}` })
			}
		} catch (e) {
			toast({ kind: 'err', title: '重命名失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	async function newFolder() {
		const base = cwd ? `${cwd}/新建文件夹` : '新建文件夹'
		let path = base
		let n = 2
		while ((tree?.dirs ?? []).includes(path)) {
			path = `${base} ${n++}`
		}
		try {
			await makeStoreDir(path)
			// 展开当前目录让新文件夹可见，并直接进入重命名
			const s = new Set(expanded)
			if (cwd) s.add(cwd)
			expanded = s
			await refresh()
			await startRename(path)
		} catch (e) {
			toast({ kind: 'err', title: '新建文件夹失败', msg: e instanceof Error ? e.message : String(e) })
		}
	}

	// ---------- 拖出（网格 → 画布 / 库内移动） ----------
	const DRAG_MIME = STORE_DRAG_MIME
	function onTileDragStart(e: DragEvent, path: string) {
		if (!e.dataTransfer) return
		// 拖未选中项 = 单独拖它；拖已选中项 = 拖整组
		let paths: string[]
		if (selected.has(path)) paths = entries.filter((entry) => selected.has(entry.path)).map((entry) => entry.path)
		else {
			selectOnly(path)
			paths = [path]
		}
		paths = paths.filter((p) => entries.some((en) => en.path === p))
		if (!paths.length) return
		e.dataTransfer.setData(DRAG_MIME, JSON.stringify({ paths }))
		const images = paths.flatMap((p) => {
			const file = fileOf(p)
			return file ? [{ kind: 'store' as const, url: storeUrl(p), store: '', file: p, w: file.w, h: file.h }] : []
		})
		if (images.length) writeImageDrag(e.dataTransfer, images)
		e.dataTransfer.effectAllowed = 'copyMove'
	}

	// ---------- 框选 ----------
	let down: { x: number; y: number; additive: boolean; moved: boolean } | null = null
	function onGridPointerDown(e: PointerEvent) {
		if (e.button !== 0) return
		// 点在 tile 上由 tile 自己处理
		if ((e.target as HTMLElement).closest('[data-entry]')) return
		down = { x: e.clientX, y: e.clientY, additive: e.ctrlKey || e.metaKey || e.shiftKey, moved: false }
		window.addEventListener('pointermove', onMarqueeMove)
		window.addEventListener('pointerup', onMarqueeUp)
	}
	function onMarqueeMove(e: PointerEvent) {
		if (!down || !gridEl) return
		if (!down.moved && Math.hypot(e.clientX - down.x, e.clientY - down.y) < 4) return
		down.moved = true
		const r = gridEl.getBoundingClientRect()
		marquee = {
			x: Math.min(down.x, e.clientX) - r.left,
			y: Math.min(down.y, e.clientY) - r.top,
			x1: Math.max(down.x, e.clientX) - r.left,
			y1: Math.max(down.y, e.clientY) - r.top,
		}
		const inRect = (el: Element) => {
			const b = el.getBoundingClientRect()
			return !(b.right < Math.min(down!.x, e.clientX) || b.left > Math.max(down!.x, e.clientX) || b.bottom < Math.min(down!.y, e.clientY) || b.top > Math.max(down!.y, e.clientY))
		}
		const s = new Set<string>()
		for (const el of gridEl.querySelectorAll('[data-entry]')) {
			if (inRect(el)) s.add((el as HTMLElement).dataset.entry!)
		}
		setSel(down.additive ? new Set([...selected, ...s]) : s)
	}
	function onMarqueeUp() {
		if (down && !down.moved && !down.additive) clearSel()
		down = null
		marquee = null
		window.removeEventListener('pointermove', onMarqueeMove)
		window.removeEventListener('pointerup', onMarqueeUp)
	}

	// ---------- 键盘（仅面板悬停时接管，避免抢画布快捷键） ----------
	function onKey(e: KeyboardEvent) {
		if (!hovered) return
		const t = e.target as HTMLElement | null
		if (t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.isContentEditable)) return
		if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'a') {
			e.preventDefault()
			selectAll()
		} else if (e.key === 'Escape') {
			if (renaming) {
				renaming = null
			} else if (ctx) {
				ctx = null
			} else {
				clearSel()
			}
		} else if (e.key === 'Delete' || e.key === 'Backspace') {
			if (selCount) {
				e.preventDefault()
				void remove([...selected])
			}
		} else if (e.key === 'F2') {
			if (selCount === 1) {
				e.preventDefault()
				void startRename([...selected][0])
			}
		} else if (e.key === 'Enter') {
			if (selCount === 1) {
				const p = [...selected][0]
				if (!fileOf(p)) {
					e.preventDefault()
					navigate(p)
				}
			}
		}
	}
	onMount(() => window.addEventListener('keydown', onKey))
	onDestroy(() => window.removeEventListener('keydown', onKey))

	// ---------- 右键菜单 ----------
	function openCtx(e: MouseEvent, kind: 'tile' | 'blank', path: string) {
		e.preventDefault()
		if (kind === 'tile' && !selected.has(path)) selectOnly(path)
		ctx = { x: e.clientX, y: e.clientY, kind, path }
	}
	function closeCtx() {
		ctx = null
	}
	function onWindowPointerDownCapture(e: PointerEvent) {
		if (ctx && !(e.target as HTMLElement).closest('.ctx-menu')) closeCtx()
	}

	// ---------- 面板高度拖拽 ----------
	let resizeStart: { y: number; h: number } | null = null
	function onResizeDown(e: PointerEvent) {
		e.preventDefault()
		resizeStart = { y: e.clientY, h: panelH }
		window.addEventListener('pointermove', onResizeMove)
		window.addEventListener('pointerup', onResizeUp)
	}
	function onResizeMove(e: PointerEvent) {
		if (!resizeStart) return
		const max = Math.round(window.innerHeight * 0.75)
		panelH = Math.max(220, Math.min(max, resizeStart.h - (e.clientY - resizeStart.y)))
	}
	function onResizeUp() {
		resizeStart = null
		window.removeEventListener('pointermove', onResizeMove)
		window.removeEventListener('pointerup', onResizeUp)
	}

	// ---------- 网格 drop（收藏 / 上传 / 库内移动） ----------
	function readDragPaths(e: DragEvent): string[] {
		const raw = e.dataTransfer?.getData(DRAG_MIME)
		if (!raw) return []
		try {
			return (JSON.parse(raw) as { paths: string[] }).paths ?? []
		} catch {
			return []
		}
	}
	async function onGridDrop(e: DragEvent) {
		e.preventDefault()
		dropTarget = null
		const moving = readDragPaths(e)
		if (moving.length) {
			await moveInto(moving, cwd)
			return
		}
		const files = e.dataTransfer?.files
		if (files && files.length) {
			void upload(files)
			return
		}
		const url = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain')
		if (url) await collect(url)
	}

	function tileTitle(path: string, w?: number, h?: number): string {
		return `${baseName(path)}${w && h ? ` · ${w}×${h}` : ''}`
	}
</script>

<svelte:window onpointerdowncapture={onWindowPointerDownCapture} />

{#if collapsed}
	<button type="button" class="dock-rail" onclick={() => ontoggle?.()} title="展开「库」面板">
		<Icon name="layers" size={14} />
		<span>库</span>
		<Icon name="chevronDown" size={12} />
	</button>
{:else}
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<section
		class="dock"
		class:maximized
		class:dragover={dropTarget !== null}
		style="height:{maximized ? '100%' : panelH + 'px'}"
		aria-label="库"
		onmouseenter={() => (hovered = true)}
		onmouseleave={() => {
			hovered = false
			closeCtx()
		}}
	>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="resize-handle"
			class:hidden={maximized}
			onpointerdown={onResizeDown}
			title="拖拽调整高度"
		></div>

		<header class="dock-head">
			<Icon name="layers" size={14} />
			<strong>库</strong>
			{#if loading}
				<Icon name="spinner" size={13} class="spin" />
			{/if}

			<!-- 面包屑 -->
			<nav class="crumbs" aria-label="目录">
				<button type="button" class="crumb" class:cur={cwd === ''} onclick={() => navigate('')}>
					根目录
				</button>
				{#each breadcrumb as seg, i (i)}
					{@const dir = breadcrumb.slice(0, i + 1).join('/')}
					<Icon name="chevronRight" size={11} />
					<button type="button" class="crumb" class:cur={cwd === dir} onclick={() => navigate(dir)}>
						{seg}
					</button>
				{/each}
			</nav>

			<span class="grow"></span>

			{#if children}
				{@render children()}
			{/if}

			<div class="tools">
				<div class="search">
					<Icon name="search" size={12} />
					<input bind:value={query} placeholder="过滤…" spellcheck="false" />
				</div>
				<IconButton icon="folderPlus" label="新建文件夹" sm onclick={() => void newFolder()} />
				<button type="button" class="ui-btn ghost sm" onclick={() => uploadRef?.click()}>
					<Icon name="upload" size={12} />
					上传
				</button>
				<input
					bind:this={uploadRef}
					type="file"
					accept="image/*"
					multiple
					style="display:none"
					onchange={(e) => {
						void upload(e.currentTarget.files)
						e.currentTarget.value = ''
					}}
				/>
				<div class="zoom" title="缩略图大小">
					<Icon name="zoomOut" size={12} />
					<input
						type="range"
						min="72"
						max="240"
						step="8"
						bind:value={tile}
						aria-label="缩略图大小"
					/>
					<Icon name="zoomIn" size={12} />
				</div>
				<IconButton
					icon={maximized ? 'restore' : 'maximize'}
					label={maximized ? '还原面板' : '最大化面板'}
					sm
					onclick={() => (maximized = !maximized)}
				/>
				<IconButton icon="refresh" label="刷新" sm onclick={() => void refresh()} />
				<IconButton icon="chevronDown" label="收起面板" sm onclick={() => ontoggle?.()} />
			</div>
		</header>

		<div class="dock-body">
			<!-- 侧边目录树 -->
			<aside class="tree" aria-label="目录树">
				{#each treeRows as row (row.path)}
					{@const isCur = row.path === cwd}
					{@const hasKids = (dirsByParent.get(row.path) ?? []).length > 0}
					<button
						type="button"
						class="tree-row"
						class:cur={isCur}
						class:drop={dropTarget === row.path}
						style="padding-left:{6 + row.depth * 13}px"
						title={row.path || '库根目录'}
						onclick={() => navigate(row.path)}
						ondragenter={() => (dropTarget = row.path)}
						ondragover={(e) => {
							e.preventDefault()
							dropTarget = row.path
						}}
						ondragleave={() => {
							if (dropTarget === row.path) dropTarget = null
						}}
						ondrop={(e) => {
							e.preventDefault()
							const moving = readDragPaths(e)
							if (moving.length) void moveInto(moving, row.path)
							else {
								const files = e.dataTransfer?.files
								if (files?.length) {
									void upload(files)
								} else {
									const url = e.dataTransfer?.getData('text/uri-list') || e.dataTransfer?.getData('text/plain')
									if (url) {
										cwd = row.path
										void collect(url)
									}
								}
							}
							dropTarget = null
						}}
					>
						{#if hasKids}
							<span
								class="twisty"
								role="button"
								tabindex="-1"
								aria-label={isExpanded(row.path) ? '折叠' : '展开'}
								onclick={(e) => {
									e.stopPropagation()
									toggleExpand(row.path)
								}}
								onkeydown={() => {}}
							>
								<Icon name={isExpanded(row.path) ? 'chevronDown' : 'chevronRight'} size={11} />
							</span>
						{:else}
							<span class="twisty ph"></span>
						{/if}
						<Icon name={row.path === '' ? 'layers' : isExpanded(row.path) ? 'folderOpen' : 'folder'} size={13} />
						<span class="tname">{row.name}</span>
						<span class="tcount mono">{childDirCount(row.path)}</span>
					</button>
				{/each}
			</aside>

			<!-- 网格 -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				bind:this={gridEl}
				class="grid-wrap"
				class:drop={dropTarget === cwd}
				style="--lib-tile:{tile}px"
				role="listbox"
				aria-multiselectable="true"
				aria-label={cwd || '库根目录'}
				tabindex="-1"
				onpointerdown={onGridPointerDown}
				oncontextmenu={(e) => openCtx(e, 'blank', cwd)}
				ondragover={(e) => {
					e.preventDefault()
					dropTarget = cwd
				}}
				ondragleave={() => {
					if (dropTarget === cwd) dropTarget = null
				}}
				ondrop={(e) => void onGridDrop(e)}
			>
				{#if entries.length === 0}
					<div class="grid-empty">
						<Icon name="layers" size={22} />
						<span>{query ? '没有匹配的项' : cwd ? '空目录' : '库还是空的'}</span>
						<small>
							{#if query}
								换个关键词，或清空过滤
							{:else}
								跑出的满意结果，拖到这里收藏；也可以上传本地图片或新建文件夹归类
							{/if}
						</small>
					</div>
				{:else}
					<div class="grid" role="presentation">
						{#each entries as en (en.path)}
							{@const isDir = en.kind === 'dir'}
							{@const isSel = selected.has(en.path)}
							<div
								data-entry={en.path}
								class="entry"
								class:dir={isDir}
								class:sel={isSel}
								role="option"
								aria-selected={isSel}
								tabindex="-1"
								title={tileTitle(en.path, isDir ? undefined : en.f?.w, isDir ? undefined : en.f?.h)}
								draggable="true"
								onpointerdown={(e) => {
									if (e.button !== 0) return
									if (e.ctrlKey || e.metaKey) toggleSel(en.path)
									else if (e.shiftKey) selectRange(en.path)
									else if (!isSel) selectOnly(en.path)
								}}
								ondblclick={() => {
									if (isDir) navigate(en.path)
									else openLightbox(storeUrl(en.path))
								}}
								ondragstart={(e) => onTileDragStart(e, en.path)}
								oncontextmenu={(e) => {
									e.stopPropagation()
									openCtx(e, 'tile', en.path)
								}}
								ondragover={(e) => {
									if (isDir) {
										e.preventDefault()
										e.stopPropagation()
										dropTarget = en.path
									}
								}}
								ondragleave={() => {
									if (isDir && dropTarget === en.path) dropTarget = null
								}}
								ondrop={(e) => {
									if (!isDir) return
									e.preventDefault()
									e.stopPropagation()
									const moving = readDragPaths(e)
									if (moving.length) void moveInto(moving, en.path)
									dropTarget = null
								}}
							>
								<div class="thumb" style="--tile:{tile}px">
									{#if isDir}
										<Icon name={dropTarget === en.path ? 'folderOpen' : 'folder'} size={Math.round(tile * 0.34)} />
										<span class="dir-badge">{childDirCount(en.path)}</span>
									{:else}
										<img src={storeUrl(en.path)} alt={baseName(en.path)} loading="lazy" draggable="false" />
									{/if}
								</div>
								{#if renaming === en.path}
									<!-- svelte-ignore a11y_autofocus -->
									<input
										bind:this={renameEl}
										bind:value={renameValue}
										class="rename"
										spellcheck="false"
										onkeydown={(e) => {
											e.stopPropagation()
											if (e.key === 'Enter') void commitRename()
											if (e.key === 'Escape') renaming = null
										}}
										onblur={() => void commitRename()}
									/>
								{:else}
									<span class="ename" title={baseName(en.path)}>{baseName(en.path)}</span>
								{/if}
							</div>
						{/each}
					</div>
				{/if}

				{#if marquee}
					<div
						class="marquee"
						style="left:{marquee.x}px;top:{marquee.y}px;width:{marquee.x1 - marquee.x}px;height:{marquee.y1 - marquee.y}px"
					></div>
				{/if}
			</div>
		</div>

		<!-- 状态栏 -->
		<footer class="dock-foot">
			<span class="mono">{totalHere} 项</span>
			<span class="sep"></span>
			<span class="mono" class:hl={selCount > 0}>选中 {selCount}</span>
			{#if selCount > 0}
				<span class="sep"></span>
				<span class="mono">{fmtSize(selBytes)}</span>
			{/if}
			<span class="grow"></span>
			<span class="hint">拖到画布即引用 · 拖到树目录即移动 · 双击预览 · Ctrl/Shift 多选 · 拖拽框选</span>
		</footer>

		<!-- 右键菜单 -->
		{#if ctx}
			{@const selPaths = ctx.kind === 'tile' && selected.has(ctx.path) ? [...selected] : [ctx.path]}
			<div
				class="ctx-menu"
				style="left:{Math.min(ctx.x, window.innerWidth - 200)}px;top:{Math.min(ctx.y, window.innerHeight - 40 - (ctx.kind === 'tile' ? 4 : 6) * 30)}px"
				role="menu"
			>
				{#if ctx.kind === 'tile'}
					{#if !fileOf(ctx.path)}
						<button
							type="button"
							role="menuitem"
							onclick={() => {
								navigate(ctx!.path)
								closeCtx()
							}}
						>
							<Icon name="folderOpen" size={13} />打开
						</button>
					{/if}
					<button
						type="button"
						role="menuitem"
						onclick={() => {
							void startRename(ctx!.path)
							closeCtx()
						}}
					>
						<Icon name="pencil" size={13} />重命名{selPaths.length > 1 ? '（首项）' : ''}
					</button>
					<button
						type="button"
						role="menuitem"
						onclick={() => {
							void moveInto(selPaths, cwd)
							closeCtx()
						}}
					>
						<Icon name="move" size={13} />移到此目录
					</button>
					<div class="ctx-sep"></div>
					<button
						type="button"
						role="menuitem"
						class:danger={true}
						onclick={() => {
							void remove(selPaths)
							closeCtx()
						}}
					>
						<Icon name="trash" size={13} />删除{selPaths.length > 1 ? ` ${selPaths.length} 项` : ''}
					</button>
				{:else}
					<button
						type="button"
						role="menuitem"
						onclick={() => {
							void newFolder()
							closeCtx()
						}}
					>
						<Icon name="folderPlus" size={13} />新建文件夹
					</button>
					<button
						type="button"
						role="menuitem"
						onclick={() => {
							uploadRef?.click()
							closeCtx()
						}}
					>
						<Icon name="upload" size={13} />上传图片
					</button>
					<div class="ctx-sep"></div>
					<button
						type="button"
						role="menuitem"
						onclick={() => {
							selectAll()
							closeCtx()
						}}
					>
						<Icon name="check" size={13} />全选
					</button>
					{#if selCount > 0}
						<button
							type="button"
							role="menuitem"
							class:danger={true}
							onclick={() => {
								void remove([...selected])
								closeCtx()
							}}
						>
							<Icon name="trash" size={13} />删除选中 {selCount} 项
						</button>
					{/if}
				{/if}
			</div>
		{/if}
	</section>
{/if}

<style>
	/* ---- 收起态：一条细轨 ---- */
	.dock-rail {
		position: fixed;
		left: 50%;
		bottom: 0;
		transform: translateX(-50%);
		display: flex;
		align-items: center;
		gap: 6px;
		height: 22px;
		padding: 0 14px;
		border: none;
		border-radius: var(--ui-r-menu) var(--ui-r-menu) 0 0;
		background: var(--ui-panel);
		border: 1px solid var(--ui-border-fade);
		border-bottom: none;
		color: var(--ui-dim);
		font: inherit;
		font-size: 11px;
		cursor: pointer;
		z-index: 40;
	}
	.dock-rail:hover {
		color: var(--ui-text);
	}
	.dock-rail :global(.ui-icon:first-child) {
		color: var(--ui-accent);
	}

	/* ---- 面板 ---- */
	.dock {
		position: relative;
		flex: none;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--ui-border-fade);
		background: var(--ui-panel);
	}
	.dock.maximized {
		position: fixed;
		inset: 0;
		z-index: 60;
		height: 100% !important;
		border-top: none;
	}
	.resize-handle {
		position: absolute;
		top: -3px;
		left: 0;
		right: 0;
		height: 6px;
		cursor: row-resize;
		z-index: 2;
	}
	.resize-handle:hover::after,
	.resize-handle:active::after {
		content: '';
		position: absolute;
		left: 50%;
		top: 2px;
		transform: translateX(-50%);
		width: 44px;
		height: 3px;
		border-radius: 2px;
		background: var(--ui-accent);
	}
	.resize-handle.hidden {
		display: none;
	}

	/* ---- 头部 ---- */
	.dock-head {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 38px;
		padding: 0 12px;
		flex: none;
		border-bottom: 1px solid var(--ui-border-fade);
	}
	.dock-head :global(.ui-icon:first-of-type) {
		color: var(--ui-accent);
	}
	.dock-head strong {
		font-size: 12px;
		font-weight: 650;
		flex: none;
	}
	.grow {
		flex: 1;
	}
	.crumbs {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
		overflow: hidden;
	}
	.crumbs :global(.ui-icon) {
		color: var(--ui-faint);
		flex: none;
	}
	.crumb {
		border: none;
		background: none;
		padding: 2px 6px;
		border-radius: var(--ui-r-control);
		color: var(--ui-dim);
		font: inherit;
		font-size: 11px;
		white-space: nowrap;
		cursor: pointer;
	}
	.crumb:hover {
		background: var(--ui-input);
		color: var(--ui-text);
	}
	.crumb.cur {
		color: var(--ui-text);
		font-weight: 600;
	}
	.tools {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}
	.search {
		display: flex;
		align-items: center;
		gap: 5px;
		height: 24px;
		padding: 0 8px;
		border-radius: var(--ui-r-control);
		background: var(--ui-input);
		border: 1px solid var(--ui-border-fade);
		color: var(--ui-faint);
	}
	.search:focus-within {
		border-color: var(--ui-accent);
	}
	.search input {
		width: 92px;
		border: none;
		outline: none;
		background: none;
		color: var(--ui-text);
		font: inherit;
		font-size: 11px;
	}
	.zoom {
		display: flex;
		align-items: center;
		gap: 5px;
		color: var(--ui-faint);
	}
	.zoom input[type='range'] {
		width: 72px;
		accent-color: var(--ui-accent);
	}

	/* ---- 主体：树 + 网格 ---- */
	.dock-body {
		flex: 1;
		min-height: 0;
		display: flex;
	}
	.dock.dragover .dock-body {
		background: var(--ui-accent-weak);
	}
	.tree {
		flex: none;
		width: 188px;
		overflow: auto;
		padding: 8px 6px;
		border-right: 1px solid var(--ui-border-fade);
	}
	.tree-row {
		display: flex;
		align-items: center;
		gap: 4px;
		width: 100%;
		height: 26px;
		padding-right: 6px;
		border: none;
		border-radius: var(--ui-r-control);
		background: none;
		color: var(--ui-dim);
		font: inherit;
		font-size: 11.5px;
		text-align: left;
		cursor: pointer;
	}
	.tree-row:hover {
		background: var(--ui-input);
		color: var(--ui-text);
	}
	.tree-row.cur {
		background: var(--ui-accent-weak);
		color: var(--ui-accent);
		font-weight: 600;
	}
	.tree-row.drop {
		background: var(--ui-accent-weak);
		box-shadow: inset 0 0 0 1.5px var(--ui-accent);
	}
	.tree-row .twisty {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 14px;
		height: 14px;
		flex: none;
		border-radius: 3px;
		color: var(--ui-faint);
		cursor: pointer;
	}
	.tree-row .twisty:hover {
		background: var(--ui-input);
		color: var(--ui-text);
	}
	.tree-row .twisty.ph {
		cursor: default;
	}
	.tree-row :global(.ui-icon) {
		flex: none;
	}
	.tree-row .tname {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.tree-row .tcount {
		flex: none;
		font-size: 9.5px;
		color: var(--ui-faint);
	}

	/* ---- 网格 ---- */
	.grid-wrap {
		flex: 1;
		min-width: 0;
		overflow: auto;
		position: relative;
		transition: background var(--ui-fast), box-shadow var(--ui-fast);
	}
	.grid-wrap.drop {
		background: var(--ui-accent-weak);
		box-shadow: inset 0 0 0 2px var(--ui-accent);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(var(--lib-tile), 1fr));
		gap: 10px;
		padding: 12px;
		align-content: start;
	}
	.grid-empty {
		position: absolute;
		inset: 0;
		margin: auto;
		height: fit-content;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 5px;
		color: var(--ui-faint);
		text-align: center;
		pointer-events: none;
	}
	.grid-empty span {
		font-size: 12px;
		font-weight: 600;
		color: var(--ui-dim);
	}
	.grid-empty small {
		font-size: 10.5px;
	}
	.entry {
		margin: 0;
		border-radius: var(--ui-r-control);
		background: var(--ui-card);
		border: 1px solid var(--ui-border-fade);
		overflow: hidden;
		user-select: none;
		transition: border-color var(--ui-fast), transform var(--ui-fast) var(--ui-ease),
			box-shadow var(--ui-fast);
	}
	.entry:hover {
		border-color: var(--ui-accent);
	}
	.entry.sel {
		border-color: var(--ui-accent);
		box-shadow: 0 0 0 1.5px var(--ui-accent);
	}
	.entry .thumb {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		height: var(--tile);
		background: var(--ui-input);
		color: var(--ui-faint);
		cursor: grab;
	}
	.entry.dir .thumb {
		cursor: pointer;
	}
	.entry .thumb img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
		pointer-events: none;
	}
	.entry .dir-badge {
		position: absolute;
		right: 5px;
		bottom: 5px;
		padding: 1px 6px;
		border-radius: 999px;
		background: var(--ui-panel);
		border: 1px solid var(--ui-border-fade);
		font-size: 9.5px;
		color: var(--ui-dim);
	}
	.entry .ename {
		padding: 4px 7px;
		font-size: 10.5px;
		color: var(--ui-dim);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.entry.sel .ename {
		color: var(--ui-text);
	}
	.entry .rename {
		width: 100%;
		border: none;
		outline: none;
		padding: 4px 6px;
		background: var(--ui-input);
		color: var(--ui-text);
		font: inherit;
		font-size: 10.5px;
	}
	.marquee {
		position: absolute;
		z-index: 5;
		border: 1px solid var(--ui-accent);
		background: var(--ui-accent-weak);
		pointer-events: none;
	}

	/* ---- 状态栏 ---- */
	.dock-foot {
		display: flex;
		align-items: center;
		gap: 8px;
		height: 26px;
		padding: 0 12px;
		flex: none;
		border-top: 1px solid var(--ui-border-fade);
		font-size: 10.5px;
		color: var(--ui-dim);
	}
	.dock-foot .mono {
		font-family: var(--ui-mono, monospace);
	}
	.dock-foot .hl {
		color: var(--ui-accent);
	}
	.dock-foot .sep {
		width: 1px;
		height: 10px;
		background: var(--ui-border-fade);
	}
	.dock-foot .hint {
		color: var(--ui-faint);
	}

	/* ---- 右键菜单 ---- */
	.ctx-menu {
		position: fixed;
		z-index: 80;
		min-width: 168px;
		padding: 5px;
		border-radius: var(--ui-r-menu);
		background: var(--ui-panel);
		border: 1px solid var(--ui-border-fade);
		box-shadow: var(--ui-shadow, 0 8px 24px rgba(0, 0, 0, 0.35));
	}
	.ctx-menu button {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		height: 28px;
		padding: 0 8px;
		border: none;
		border-radius: var(--ui-r-control);
		background: none;
		color: var(--ui-text);
		font: inherit;
		font-size: 11.5px;
		text-align: left;
		cursor: pointer;
	}
	.ctx-menu button:hover {
		background: var(--ui-input);
	}
	.ctx-menu button.danger {
		color: var(--ui-danger, #e5484d);
	}
	.ctx-sep {
		height: 1px;
		margin: 4px 6px;
		background: var(--ui-border-fade);
	}
</style>
