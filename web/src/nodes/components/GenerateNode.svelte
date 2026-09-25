<script lang="ts">
	import { Ref } from 'rete-svelte-plugin/5'
	import type { AreaExtra } from '../types'
	import { rt } from '../../runtime'
	import { scheduleSave } from '../../persist'
	import { OPENAI_IMAGE_PARAMS } from '../../apiParams'
	import type { ParamDef } from '../../profiles'
	import type { GenerateNode } from '../classes'

	export let data: GenerateNode
	export let emit: (p: AreaExtra) => void

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

	$: mainParams = OPENAI_IMAGE_PARAMS.filter((p) => !p.advanced)
	$: advancedParams = OPENAI_IMAGE_PARAMS.filter((p) => p.advanced)
</script>

<div class="an-node" class:selected={data.selected} data-node-id={data.id}>
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
		{#if p.kind === 'select' && p.control === 'slider'}
			{@const opts = ['', ...p.options]}
			{@const idx = Math.max(0, opts.indexOf(String(data.params[p.key] ?? '')))}
			<div class="an-slider">
				<input
					type="range"
					min="0"
					max={opts.length - 1}
					step="1"
					value={idx}
					title={String(data.params[p.key] ?? '默认')}
					on:pointerdown|stopPropagation
					on:input={(e) => setParam(p, opts[Number(e.currentTarget.value)])}
				/>
				<div class="an-ticks">
					{#each opts as o, i (i)}
						<button
							type="button"
							class="an-tick"
							class:active={idx === i}
							style:left="{(i / (opts.length - 1)) * 100}%"
							title={o === '' ? '默认' : o}
							on:pointerdown|stopPropagation
							on:click={() => setParam(p, o)}
						></button>
					{/each}
				</div>
			</div>
			<span class="an-value">{data.params[p.key] ?? '默认'}</span>
		{:else if p.kind === 'select' && p.control === 'segmented'}
			<div class="an-seg">
				{#each ['', ...p.options] as o, i (i)}
					<button
						type="button"
						class:active={(data.params[p.key] ?? '') === o}
						on:pointerdown|stopPropagation
						on:click={() => setParam(p, o)}
					>
						{o === '' ? '默认' : o}
					</button>
				{/each}
			</div>
		{:else if p.kind === 'select'}
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
		{:else if p.kind === 'size'}
			<!-- 协议开集合：预设建议 + 自由输入（gpt-image-2+ 任意 16 整除尺寸） -->
			<input
				type="text"
				list="an-size-presets"
				placeholder="默认"
				value={data.params[p.key] ?? ''}
				on:pointerdown|stopPropagation
				on:change={(e) => setParam(p, e.currentTarget.value.trim())}
			/>
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

<datalist id="an-size-presets">
	{#each OPENAI_IMAGE_PARAMS.find((p) => p.key === 'size')?.options ?? [] as o (o)}
		<option value={o}></option>
	{/each}
</datalist>
