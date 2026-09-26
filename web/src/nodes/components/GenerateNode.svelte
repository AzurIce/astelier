<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import Icon from '../../components/Icon.svelte'
	import type { AreaExtra } from '../types'
	import { OPENAI_IMAGE_PARAMS } from '../../apiParams'
	import { openLightbox } from '../../components/lightbox.svelte'
	import type { ParamDef } from '../../profiles'
	import { editNode, removeNodeCascade } from '../actions'
	import type { GenerateNode } from '../classes'

	let { data, emit }: { data: GenerateNode; emit: (p: AreaExtra) => void } = $props()

	let mainParams = $derived(OPENAI_IMAGE_PARAMS.filter((p) => !p.advanced))
	let advancedParams = $derived(OPENAI_IMAGE_PARAMS.filter((p) => p.advanced))

	function val(p: ParamDef): string {
		const v = data.params[p.key]
		return v === undefined ? '' : String(v)
	}
	function setParam(p: ParamDef, raw: string) {
		editNode<GenerateNode>(data.id, (n) => {
			if (raw === '') {
				delete n.params[p.key]
			} else if (p.kind === 'number') {
				const v = Number(raw)
				if (isNaN(v)) return
				n.params[p.key] = v
			} else {
				n.params[p.key] = raw
			}
		})
	}
	function stop(e: PointerEvent) {
		e.stopPropagation()
	}
	const sizePresets = OPENAI_IMAGE_PARAMS.find((p) => p.key === 'size')?.options ?? []
</script>

<NodeFrame
	nodeId={data.id}
	type="generate"
	icon="generate"
	name="Generate"
	desc={data.busy ? '生成中…' : 'OpenAI Images 协议'}
	selected={data.selected}
	busy={data.busy}
	ondelete={() => void removeNodeCascade(data.id)}
>
	{#snippet inputs()}
		<Port {data} {emit} side="input" port="model" label="model" tone="model" />
		<Port {data} {emit} side="input" port="prompt" label="prompt ×" tone="text" />
		<Port {data} {emit} side="input" port="image" label="ref ×" tone="image" />
	{/snippet}

	{#snippet body()}
		{#each mainParams as p (p.key)}
			{@render ParamRow(p)}
		{/each}

		{#if advancedParams.length}
			<details class="ui-details">
				<summary><Icon name="chevronDown" size={12} />更多参数</summary>
				<div class="details-body">
					{#each advancedParams as p (p.key)}
						{@render ParamRow(p)}
					{/each}
				</div>
			</details>
		{/if}

		{#if data.busy}
			<div class="gen-status">
				<div class="ui-status">
					<Icon name="spinner" size={13} class="spin" />
					<span>生成中，可能需要几十秒…</span>
				</div>
				<div class="ui-skeleton"></div>
				<div class="ui-skeleton" style="width: 62%"></div>
			</div>
		{:else if data.error}
			<div class="ui-error" title={data.error}>
				<Icon name="alert" size={13} />
				<span>{data.error}</span>
			</div>
		{/if}

		{#if data.resultUrl && !data.busy}
			<button
				type="button"
				class="img-btn"
				title="点击查看大图"
				onpointerdown={stop}
				onclick={() => data.resultUrl && openLightbox(data.resultUrl)}
			>
				<img class="ui-img thumbnail result-img" src={data.resultUrl} alt="生成结果" />
			</button>
		{/if}
	{/snippet}

	{#snippet outputs()}
		<Port {data} {emit} side="output" port="image" label="image" tone="image" />
	{/snippet}
</NodeFrame>

<datalist id="size-presets">
	{#each sizePresets as o (o)}
		<option value={o}></option>
	{/each}
</datalist>

{#snippet ParamRow(p: ParamDef)}
	<div class="field-row">
		<span class="field-label" title={p.key}>{p.label}</span>
		<div class="field-value">
			{#if p.kind === 'select' && p.control === 'slider'}
				{@const opts = ['', ...p.options]}
				{@const idx = Math.max(0, opts.indexOf(val(p)))}
				<div class="slider-wrap">
					<div class="top">
						<span class="mono">{val(p) || '默认'}</span>
					</div>
					<input
						class="ui-range"
						type="range"
						min="0"
						max={opts.length - 1}
						step="1"
						value={idx}
						title={val(p) || '默认'}
						onpointerdown={stop}
						oninput={(e) => setParam(p, opts[Number((e.target as HTMLInputElement).value)])}
					/>
				</div>
			{:else if p.kind === 'select' && p.control === 'segmented'}
				<div class="ui-seg">
					{#each ['', ...p.options] as o (o)}
						<button
							type="button"
							class:active={val(p) === o}
							onpointerdown={stop}
							onclick={() => setParam(p, o)}
						>
							{o === '' ? '默认' : o}
						</button>
					{/each}
				</div>
			{:else if p.kind === 'select'}
				<select
					class="ui-select"
					value={val(p)}
					title={val(p) || '默认'}
					onpointerdown={stop}
					onchange={(e) => setParam(p, (e.target as HTMLSelectElement).value)}
				>
					<option value="">默认</option>
					{#each p.options as o (o)}
						<option value={o}>{o}</option>
					{/each}
				</select>
			{:else if p.kind === 'size'}
				<input
					class="ui-input mono"
					type="text"
					list="size-presets"
					placeholder="默认"
					title="常用尺寸可从下拉选择，也可直接输入如 1216x832（16 整除）"
					value={val(p)}
					onpointerdown={stop}
					onchange={(e) => setParam(p, (e.target as HTMLInputElement).value.trim())}
				/>
			{:else if p.kind === 'number'}
				<div class="ui-stepper">
					<button
						type="button"
						aria-label="减少"
						onpointerdown={stop}
						onclick={() => {
							const cur = Number(val(p) || p.min || 0)
							const step = Math.max(1, Math.floor(cur) - 1)
							setParam(p, String(Math.max(p.min ?? 0, step)))
						}}
					>
						−
					</button>
					<input
						class="ui-num"
						type="number"
						min={p.min ?? undefined}
						max={p.max ?? undefined}
						value={val(p)}
						placeholder="默认"
						onpointerdown={stop}
						onchange={(e) => setParam(p, (e.target as HTMLInputElement).value)}
					/>
					<button
						type="button"
						aria-label="增加"
						onpointerdown={stop}
						onclick={() => {
							const cur = Number(val(p) || p.min || 0)
							const step = Math.ceil(cur) + 1
							setParam(p, String(Math.min(p.max ?? 99, step)))
						}}
					>
						+
					</button>
				</div>
			{:else}
				<input
					class="ui-input"
					type="text"
					value={val(p)}
					placeholder="默认"
					onpointerdown={stop}
					onchange={(e) => setParam(p, (e.target as HTMLInputElement).value)}
				/>
			{/if}
		</div>
	</div>
{/snippet}

<style>
	.gen-status {
		display: flex;
		flex-direction: column;
		gap: 6px;
	}
	.result-img {
		margin: 2px auto 0;
	}
	.slider-wrap .top .mono {
		color: var(--ui-dim);
		font-size: 10.5px;
	}
</style>
