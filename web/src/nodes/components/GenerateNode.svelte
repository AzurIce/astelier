<script lang="ts">
	import { onMount } from 'svelte'
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt, modelVersion } from '../../runtime'
	import { scheduleSave } from '../../persist'
	import { connectedModelNode } from '../conn'
	import { fetchProfiles, type ModelProfile, type ParamDef } from '../../profiles'
	import type { GenerateNode } from '../classes'

	export let data: GenerateNode
	export let emit: (p: AreaExtra) => void

	let ready = false
	let profiles: ModelProfile[] = []
	let loadedFor = ''
	let loadError: string | null = null

	async function syncProfiles() {
		const src = connectedModelNode(data.id)
		const pid = src?.provider ?? ''
		if (pid === loadedFor) return
		loadedFor = pid
		profiles = []
		if (!pid) return
		try {
			profiles = await fetchProfiles(pid)
			loadError = null
		} catch (e) {
			loadError = e instanceof Error ? e.message : String(e)
		}
	}

	// 触发时机：挂载 + Model 选择/连线变化（modelVersion 信号）
	$: if (ready) syncProfilesWhenChanged($modelVersion)
	function syncProfilesWhenChanged(_version: number) {
		syncProfiles()
	}
	onMount(() => {
		ready = true
		syncProfiles()
	})

	function touch() {
		rt.area?.update('node', data.id)
		scheduleSave()
	}

	function setParam(p: ParamDef, raw: string) {
		if (raw === '') {
			delete data.params[p.key]
		} else if (p.kind === 'number') {
			const n = Number(raw)
			if (isNaN(n)) return
			data.params[p.key] = n
		} else {
			data.params[p.key] = raw
		}
		touch()
	}

	$: mainParams = profiles.flatMap((pr) => pr.params).filter((p) => !p.advanced)
	$: advancedParams = profiles.flatMap((pr) => pr.params).filter((p) => p.advanced)
</script>

<div class="an-node" class:selected={data.selected}>
	<div class="an-title an-t-model">Generate</div>

	<div class="an-in">
		<Ref
			class="an-socket an-sock-model"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'model',
						nodeId: data.id,
						element,
						payload: data.inputs.model!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">model</span>
	</div>
	<div class="an-in">
		<Ref
			class="an-socket an-sock-text"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'prompt',
						nodeId: data.id,
						element,
						payload: data.inputs.prompt!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">prompt ×</span>
	</div>
	<div class="an-in">
		<Ref
			class="an-socket an-sock-image"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'input',
						key: 'image',
						nodeId: data.id,
						element,
						payload: data.inputs.image!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
		<span class="an-port-label">ref ×</span>
	</div>

	<div class="an-body">
		{#if !profiles.length && !loadError}
			<div class="an-dim">连接 Model 节点以加载参数</div>
		{:else if loadError}
			<div class="an-error">{loadError}</div>
		{/if}

		{#each mainParams as p (p.key)}
			{@render ParamRow(p)}
		{/each}

		{#if advancedParams.length}
			<details class="an-advanced">
				<summary>更多参数</summary>
				{#each advancedParams as p (p.key)}
					{@render ParamRow(p)}
				{/each}
			</details>
		{/if}

		{#if data.busy}
			<div class="an-status">生成中…</div>
		{:else if data.error}
			<div class="an-error" title={data.error}>{data.error}</div>
		{/if}
		{#if data.resultUrl}
			<img class="an-preview" src={data.resultUrl} alt="生成结果" />
		{/if}
	</div>

	<div class="an-out">
		<span class="an-port-label">image</span>
		<Ref
			class="an-socket an-sock-image"
			init={(element: HTMLElement) =>
				emit({
					type: 'render',
					data: {
						type: 'socket',
						side: 'output',
						key: 'image',
						nodeId: data.id,
						element,
						payload: data.outputs.image!.socket,
					},
				})}
			unmount={(ref: HTMLElement) => emit({ type: 'unmount', data: { element: ref } })}
		/>
	</div>
</div>

{#snippet ParamRow(p: ParamDef)}
	<div class="an-row">
		<span class="an-label" title={p.key}>{p.label}</span>
		{#if p.kind === 'select' || p.kind === 'size'}
			<select
				value={data.params[p.key] ?? ''}
				on:pointerdown|stopPropagation
				on:change={(e) => setParam(p, e.currentTarget.value)}
			>
				<option value="">默认</option>
				{#each p.options as o (o)}
					<option value={o}>{o}</option>
				{/each}
			</select>
		{:else}
			<input
				type={p.kind === 'number' ? 'number' : 'text'}
				min={p.min ?? undefined}
				max={p.max ?? undefined}
				value={data.params[p.key] ?? ''}
				on:pointerdown|stopPropagation
				on:change={(e) => setParam(p, e.currentTarget.value)}
			/>
		{/if}
	</div>
{/snippet}
