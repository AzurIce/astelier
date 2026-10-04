<script lang="ts">
	import { onDestroy } from 'svelte'
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import Icon from '../../components/Icon.svelte'
	import type { AreaExtra } from '../types'
	import { toast } from '../../components/toast.svelte'
	import { openLightbox } from '../../components/lightbox.svelte'
	import { editNode, releaseGraphStoreFiles, removeNodeCascade } from '../actions'
	import { noNodeDrag } from '../noNodeDrag'
	import { noCanvasWheel } from '../../noCanvasWheel'
	import { acceptImageDrop } from '../acceptImageDrop'
	import { activeGraphId, graphSession, uploadGraphStoreFile, graphStoreUrl } from '../../graphStore.svelte'
	import { rt } from '../../runtime'
	import { LoadImageNode, type ImageRef } from '../classes.svelte'
	import { IMAGE_ORDER_MIME, writeImageDrag, type DragImagePayload } from '../dragPayload'
	import { createImageImporter, type ImportProgress, type ImportReport, type ImageSource } from '../imageImport'

	let { data, emit }: { data: LoadImageNode; emit: (p: AreaExtra) => void } = $props()
	let input: HTMLInputElement | undefined = $state()
	let over = $state(false)
	let progress: ImportProgress | null = $state(null)
	let report: ImportReport | null = $state(null)
	let dragging = $state<string | null>(null)
	let sortOver = $state<string | null>(null)
	let broken = $state(new Set<string>())
	const busy = $derived(progress !== null)
	const images = $derived(data.images ?? [])
	const title = $derived(images.length ? `参考图 · ${images.length} 张` : '参考图 / 垫图')

	const importer = createImageImporter({
		target: () => {
			const graphId = activeGraphId()
			const node = rt.editor?.getNode(data.id)
			return {
				graphId,
				isActive: () => !graphSession.loading && activeGraphId() === graphId && node instanceof LoadImageNode && rt.editor?.getNode(node.id) === node,
				images: () => node instanceof LoadImageNode ? node.images : [],
				append: (image) => {
					let added = false
					editNode<LoadImageNode>(data.id, (node) => { added = node.addImage(image) })
					return added
				},
			}
		},
		upload: uploadGraphStoreFile,
		onProgress: (next) => { progress = next },
		onReport: (next) => {
			report = next
			if (next.failures.length) toast({ kind: 'err', title: `${next.failures.length} 张图片未能添加`, msg: `已添加 ${next.added} 张，可在节点中查看原因并重试` })
		},
	})
	onDestroy(() => importer.dispose())

	function enqueue(sources: ImageSource[]) {
		report = null
		void importer.enqueue(sources)
	}
	function addFiles(files: File[]) {
		enqueue(files.map((file) => ({ kind: 'file', file })))
	}
	function addUrls(images: DragImagePayload[]) {
		enqueue(images.map((image) => ({ kind: 'url', image })))
	}
	function remove(file: string) {
		const persistent = !images.find((image) => image.file === file)?.dataUrl
		editNode<LoadImageNode>(data.id, (node) => node.removeImage(file))
		if (persistent) void releaseGraphStoreFiles([file])
		broken = new Set([...broken].filter((name) => name !== file))
	}
	function clearAll() {
		const files = images.filter((image) => !image.dataUrl).map((image) => image.file)
		editNode<LoadImageNode>(data.id, (node) => { node.images = [] })
		void releaseGraphStoreFiles(files)
		report = null
		broken = new Set()
	}
	function move(file: string, index: number) {
		editNode<LoadImageNode>(data.id, (node) => node.moveImage(file, index))
	}
	function sortKey(e: KeyboardEvent, file: string, index: number) {
		if (e.key === 'Delete' || e.key === 'Backspace') {
			e.preventDefault()
			e.stopPropagation()
			if (!busy) remove(file)
			return
		}
		if (!['ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowDown', 'Home', 'End'].includes(e.key)) return
		e.preventDefault()
		e.stopPropagation()
		move(file, e.key === 'Home' ? 0 : e.key === 'End' ? images.length - 1 : index + (['ArrowLeft', 'ArrowUp'].includes(e.key) ? -1 : 1))
	}
	function startSort(e: DragEvent, file: string) {
		if (!e.dataTransfer) return
		dragging = file
		e.dataTransfer.setData(IMAGE_ORDER_MIME, JSON.stringify({ nodeId: data.id, file }))
		e.dataTransfer.effectAllowed = 'move'
		const card = (e.currentTarget as HTMLElement).closest('.image-card')
		if (card) e.dataTransfer.setDragImage(card, 24, 24)
	}
	function endSort() {
		dragging = null
		sortOver = null
	}
	function dragImage(e: DragEvent, image: ImageRef) {
		if (!e.dataTransfer) return
		writeImageDrag(e.dataTransfer, [{ kind: 'store', url: imageUrl(image), store: activeGraphId(), file: image.dataUrl ? image.name : image.file, w: image.w, h: image.h }])
		e.dataTransfer.effectAllowed = 'copy'
	}
	function imageUrl(image: ImageRef): string {
		return image.dataUrl ?? graphStoreUrl(activeGraphId(), image.file)
	}

	// 原生监听先于画布层执行，排序不会进入外部图片导入入口。
	function sortTarget(node: HTMLElement, initial: { file: string; index: number }) {
		let target = initial
		function over(e: DragEvent) {
			if (!dragging || !e.dataTransfer?.types.includes(IMAGE_ORDER_MIME)) return
			e.preventDefault()
			e.stopPropagation()
			e.dataTransfer.dropEffect = 'move'
			sortOver = target.file === dragging ? null : target.file
		}
		function leave(e: DragEvent) {
			if (e.relatedTarget instanceof Node && node.contains(e.relatedTarget)) return
			if (sortOver === target.file) sortOver = null
		}
		function drop(e: DragEvent) {
			if (!dragging || !e.dataTransfer) return
			e.preventDefault()
			e.stopPropagation()
			try {
				const payload = JSON.parse(e.dataTransfer.getData(IMAGE_ORDER_MIME))
				if (payload.nodeId === data.id && payload.file === dragging) move(payload.file, target.index)
			} catch { /* 忽略其他应用或损坏的排序载荷 */ }
			endSort()
		}
		node.addEventListener('dragover', over)
		node.addEventListener('dragleave', leave)
		node.addEventListener('drop', drop)
		return {
			update(next: { file: string; index: number }) { target = next },
			destroy() {
				node.removeEventListener('dragover', over)
				node.removeEventListener('dragleave', leave)
				node.removeEventListener('drop', drop)
			},
		}
	}
</script>

<NodeFrame nodeId={data.id} type="image" icon="image" name="Image" {title} {busy} selected={data.selected} ondelete={() => void removeNodeCascade(data.id)}>
	{#snippet body()}
		<div class="image-editor" use:noNodeDrag use:acceptImageDrop={{ onDrop: addUrls, onFiles: addFiles, onActiveChange: (active) => { over = active } }}>
			<input bind:this={input} class="file-input" type="file" accept="image/png,image/jpeg,image/webp,image/gif,.png,.jpg,.jpeg,.webp,.gif" multiple aria-label="添加参考图片" onchange={(e) => {
				addFiles(Array.from(e.currentTarget.files ?? []))
				e.currentTarget.value = ''
			}} />

			{#if images.length}
				<div class="image-toolbar">
					<span class="image-count"><Icon name="layers" size={13} />{images.length} 张参考图</span>
					<button type="button" class="add-button" onclick={() => input?.click()}><Icon name="plus" size={13} />添加</button>
				</div>
				<div class="gallery-scroll" use:noCanvasWheel>
					<ol class="image-gallery" class:single={images.length === 1} aria-label="参考图，按序号发送">
						{#each images as image, index (image.file)}
							<li class="image-card" class:sorting={dragging === image.file} class:sort-target={sortOver === image.file} use:sortTarget={{ file: image.file, index }}>
								<div class="image-preview">
									<button type="button" class="preview-button" aria-label={`预览 ${image.name}`} title="点击预览 · 拖到其他 Image 节点或图片库" draggable="true" ondragstart={(e) => dragImage(e, image)} onclick={() => openLightbox(imageUrl(image))}>
										{#if broken.has(image.file)}
											<span class="missing-image"><Icon name="alert" size={22} />图片不可用</span>
										{:else}
											<img src={imageUrl(image)} alt={image.name} draggable="false" onerror={() => { broken = new Set([...broken, image.file]) }} />
										{/if}
									</button>
									<button type="button" class="order-handle" draggable="true" aria-label={`调整 ${image.name} 的顺序，当前位置 ${index + 1}`} title="拖动排序 · 聚焦后用方向键移动" ondragstart={(e) => startSort(e, image.file)} ondragend={endSort} onkeydown={(e) => sortKey(e, image.file, index)}>
										<Icon name="grip" size={11} /><span>{String(index + 1).padStart(2, '0')}</span>
									</button>
									<button type="button" class="remove-button" aria-label={`移除 ${image.name}`} title={`移除 ${image.name}`} disabled={busy} onclick={() => remove(image.file)}><Icon name="x" size={13} /></button>
								</div>
								<div class="image-meta"><span title={image.name}>{image.name}</span><small>{image.w && image.h ? `${image.w} × ${image.h}` : '参考图片'}</small></div>
							</li>
						{/each}
					</ol>
				</div>
				<div class="gallery-footer"><span>{images.length > 1 ? '拖动序号排序 · 按顺序输出' : '拖入图片可继续添加'}</span><button type="button" class="clear-button" disabled={busy} onclick={clearAll} title="移除节点内全部参考图">清空</button></div>
			{:else}
				<button type="button" class="empty-drop" onclick={() => input?.click()}>
					<span class="empty-icon"><Icon name="image" size={26} /><span class="plus-badge"><Icon name="plus" size={10} /></span></span>
					<strong>添加参考图片</strong>
					<span>拖入多张图片，或点击选择</span>
					<small>本地文件 · 图片库 · 画布图片</small>
				</button>
			{/if}

			{#if progress}
				<div class="import-status" role="status"><span class="spin"><Icon name="spinner" size={13} /></span><span>正在添加 {progress.completed + 1} / {progress.total}</span></div>
				<progress class="import-progress" value={progress.completed} max={progress.total} aria-label="图片导入进度"></progress>
			{/if}
			{#if report?.failures.length}
				<div class="import-errors" role="status">
					<div class="error-heading"><Icon name="alert" size={13} /><span>{report.failures.length} 张未能添加</span><button type="button" onclick={() => enqueue(report!.failures.map((failure) => failure.source))}>重试</button></div>
					<ul>{#each report.failures as failure}<li><strong>{failure.name}</strong><span>{failure.message}</span></li>{/each}</ul>
				</div>
			{:else if report && report.skipped > 0}
				<div class="import-note" role="status"><Icon name="check" size={12} />{report.added ? `已添加 ${report.added} 张 · ` : ''}跳过 {report.skipped} 张重复图片</div>
			{/if}
			{#if over}
				<div class="drop-overlay"><Icon name="plus" size={25} /><strong>松开添加图片</strong><span>支持一次拖入多张</span></div>
			{/if}
		</div>
	{/snippet}
	{#snippet outputs()}<Port {data} {emit} side="output" port="image" label="image" tone="image" />{/snippet}
</NodeFrame>

<style>
	:global(.ui-node[data-node-type='image']) { width: 340px; max-width: 340px; }
	.image-editor { position: relative; display: flex; flex-direction: column; gap: 10px; min-width: 0; }
	.file-input { display: none; }
	button { font: inherit; }
	button:focus-visible { outline: 2px solid var(--ui-accent); outline-offset: 3px; }
	button:disabled { opacity: .35; cursor: default; }
	.image-toolbar { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
	.image-count { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--ui-dim); }
	.add-button { display: inline-flex; align-items: center; gap: 4px; padding: 5px 8px; color: var(--ui-accent); background: var(--ui-accent-weak); border: 0; border-radius: 6px; font-size: 11px; cursor: pointer; }
	.add-button:hover { background: var(--ui-accent-fade); }
	.gallery-scroll { max-height: 342px; overflow-y: auto; scrollbar-width: thin; scrollbar-color: var(--ui-track) transparent; padding: 2px; margin: -2px; }
	.image-gallery { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px; margin: 0; padding: 0; list-style: none; }
	.image-gallery.single { grid-template-columns: minmax(0, 1fr); }
	.image-card { min-width: 0; border: 1px solid var(--ui-border-fade); border-radius: 8px; overflow: hidden; background: var(--ui-input); transition: border-color var(--ui-fast), opacity var(--ui-fast); }
	.image-card:hover, .image-card:focus-within { border-color: var(--ui-accent-fade); }
	.image-card.sorting { opacity: .4; }
	.image-card.sort-target { border-color: var(--ui-accent); box-shadow: inset 0 0 0 2px var(--ui-accent); }
	.image-preview { position: relative; height: 108px; background-color: var(--ui-panel); background-image: conic-gradient(var(--ui-hover) 25%, transparent 0 50%, var(--ui-hover) 0 75%, transparent 0); background-size: 16px 16px; }
	.single .image-preview { height: 194px; }
	.preview-button { display: block; width: 100%; height: 100%; padding: 8px; border: 0; background: transparent; cursor: zoom-in; }
	.preview-button img { display: block; width: 100%; height: 100%; object-fit: contain; }
	.order-handle, .remove-button { position: absolute; top: 5px; display: flex; justify-content: center; align-items: center; height: 23px; border: 1px solid var(--ui-border-fade); background: var(--ui-card); color: var(--ui-dim); border-radius: 5px; box-shadow: 0 1px 4px #0002; }
	.order-handle { left: 5px; gap: 2px; padding: 0 5px 0 3px; cursor: grab; font-family: var(--ui-mono); font-size: 10px; font-variant-numeric: tabular-nums; }
	.order-handle:active { cursor: grabbing; }
	.order-handle:hover { color: var(--ui-accent); }
	.remove-button { right: 5px; width: 23px; padding: 0; cursor: pointer; opacity: 0; transition: opacity var(--ui-fast); }
	.image-card:hover .remove-button, .image-card:focus-within .remove-button { opacity: 1; }
	.remove-button:hover { color: var(--ui-danger); }
	.image-meta { display: flex; flex-direction: column; gap: 3px; padding: 7px 8px; }
	.image-meta > span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--ui-text); font-size: 10.5px; }
	.image-meta small { font: 9px var(--ui-mono); color: var(--ui-faint); }
	.gallery-footer { display: flex; align-items: center; justify-content: space-between; gap: 6px; color: var(--ui-faint); font-size: 10px; }
	.clear-button { padding: 2px 0 2px 6px; background: transparent; border: 0; color: var(--ui-dim); cursor: pointer; font-size: 10px; }
	.clear-button:hover { color: var(--ui-danger); }
	.empty-drop { display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 7px; min-height: 190px; padding: 20px 10px; border: 1px dashed var(--ui-border-fade); border-radius: 10px; color: var(--ui-dim); background: color-mix(in srgb, var(--ui-input) 65%, transparent); cursor: pointer; transition: background var(--ui-fast), border-color var(--ui-fast); }
	.empty-drop:hover { border-color: var(--ui-accent); background: var(--ui-accent-weak); }
	.empty-icon { position: relative; display: grid; place-items: center; width: 52px; height: 52px; margin-bottom: 7px; border: 1px solid var(--ui-border-fade); border-radius: 14px; color: var(--ui-sock-image); background: var(--ui-card); }
	.plus-badge { position: absolute; right: -3px; bottom: -3px; display: grid; place-items: center; width: 18px; height: 18px; border: 2px solid var(--ui-card); border-radius: 50%; background: var(--ui-sock-image); color: var(--ui-card); }
	.empty-drop strong { font-size: 12px; font-weight: 550; color: var(--ui-text); }
	.empty-drop > span:not(.empty-icon) { font-size: 11px; }
	.empty-drop small { font-size: 10px; color: var(--ui-faint); margin-top: 5px; }
	.import-status { display: flex; align-items: center; gap: 6px; color: var(--ui-accent); font-size: 10.5px; }
	.import-progress { width: 100%; height: 3px; margin-top: -6px; border: none; accent-color: var(--ui-accent); }
	.import-progress::-webkit-progress-bar { background: var(--ui-track); border-radius: 3px; }
	.import-progress::-webkit-progress-value { background: var(--ui-accent); border-radius: 3px; }
	.import-errors { padding: 8px; border: 1px solid color-mix(in srgb, var(--ui-danger) 25%, transparent); background: color-mix(in srgb, var(--ui-danger) 5%, transparent); border-radius: 7px; font-size: 10px; }
	.error-heading { display: flex; align-items: center; gap: 5px; color: var(--ui-danger); }
	.error-heading span { flex: 1; }
	.error-heading button { color: var(--ui-text); background: var(--ui-card); border: 1px solid var(--ui-border-fade); border-radius: 4px; padding: 2px 6px; cursor: pointer; }
	.import-errors ul { max-height: 88px; overflow: auto; list-style: none; margin: 7px 0 0; padding: 0; }
	.import-errors li { display: flex; flex-direction: column; gap: 2px; margin-top: 5px; overflow-wrap: anywhere; color: var(--ui-dim); }
	.import-errors strong { font-weight: 500; color: var(--ui-text); }
	.import-note { display: flex; align-items: center; gap: 4px; color: var(--ui-dim); font-size: 10px; }
	.drop-overlay { position: absolute; z-index: 3; inset: -5px; display: flex; align-items: center; justify-content: center; flex-direction: column; gap: 9px; border: 2px dashed var(--ui-accent); border-radius: 10px; color: var(--ui-accent); background: color-mix(in srgb, var(--ui-card) 94%, transparent); pointer-events: none; }
	.drop-overlay strong { font-size: 13px; }
	.drop-overlay span { font-size: 10.5px; color: var(--ui-dim); }
	.missing-image { display: flex; flex-direction: column; align-items: center; gap: 8px; color: var(--ui-faint); font-size: 11px; }
	@media (hover: none) { .remove-button { opacity: 1; } }
</style>
