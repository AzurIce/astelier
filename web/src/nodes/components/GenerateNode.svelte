<script lang="ts">
	import NodeFrame from './NodeFrame.svelte'
	import Port from './Port.svelte'
	import Icon from '../../components/Icon.svelte'
	import type { AreaExtra } from '../types'
	import { OPENAI_IMAGE_PARAMS } from '../../apiParams'
	import { openLightbox } from '../../components/lightbox.svelte'
	import type { ParamDef } from '../../profiles'
	import { editNode, removeNodeCascade } from '../actions'
	import { noNodeDrag } from '../noNodeDrag'
	import type { GenerateNode } from '../classes'

	let { data, emit }: { data: GenerateNode; emit: (p: AreaExtra) => void } = $props()
	let mainParams = $derived(OPENAI_IMAGE_PARAMS.filter((p) => !p.advanced))
	let advancedParams = $derived(OPENAI_IMAGE_PARAMS.filter((p) => p.advanced))

	function val(p: ParamDef): string {
		const v = data.params[p.key]
		// 始终完整发送：缺失键回落到协议默认（老图 / 未初始化 params）
		return v === undefined ? String(p.def ?? '') : String(v)
	}
	function setParam(p: ParamDef, raw: string) {
		editNode<GenerateNode>(data.id, (n) => {
			if (raw === '') {
				// 可清空输入（size）：空 = 回默认值，不产生「不发送」
				n.params[p.key] = typeof p.def === 'number' ? p.def : String(p.def ?? '')
			} else if (p.kind === 'number') {
				const v = Number(raw)
				if (isNaN(v)) return
				n.params[p.key] = v
			} else {
				n.params[p.key] = raw
			}
		})
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
			<!-- 展开改变节点高度：rete 不发 resize 信号，socket 位置由
			     editor.ts 的 ResizeObserver 兜底重算（下一帧合帧） -->
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
				use:noNodeDrag
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
				{@const opts = p.options}
				{@const idx = Math.max(0, opts.indexOf(val(p)))}
				<div class="slider-wrap">
					<div class="top">
						<span class="mono">{val(p)}</span>
					</div>
					<input
						class="ui-range"
						type="range"
						min="0"
						max={opts.length - 1}
						step="1"
						value={idx}
						title={val(p)}
						use:noNodeDrag
						oninput={(e) => setParam(p, opts[Number((e.target as HTMLInputElement).value)])}
					/>
					<div class="ticks">
						{#each opts as o, i (i)}
							<button
								type="button"
								class="tick"
								class:active={idx === i}
								style:left="{(i / (opts.length - 1)) * 100}%"
								title={o}
								use:noNodeDrag
								onclick={() => setParam(p, o)}
							></button>
						{/each}
					</div>
				</div>
			{:else if p.kind === 'select' && p.control === 'segmented'}
				<div class="ui-seg">
					{#each p.options as o (o)}
						<button
							type="button"
							class:active={val(p) === o}
							use:noNodeDrag
							onclick={() => setParam(p, o)}
						>
							{o}
						</button>
					{/each}
				</div>
			{:else if p.kind === 'select'}
				<select
					class="ui-select"
					value={val(p)}
					title={val(p)}
					use:noNodeDrag
					onchange={(e) => setParam(p, (e.target as HTMLSelectElement).value)}
				>
					{#each p.options as o (o)}
						<option value={o}>{o}</option>
					{/each}
				</select>
			{:else if p.kind === 'size'}
				<input
					class="ui-input mono"
					type="text"
					list="size-presets"
					placeholder={String(p.def ?? '')}
					title="常用尺寸可从下拉选择，也可直接输入如 1216x832（16 整除）"
					value={val(p)}
					use:noNodeDrag
					onchange={(e) => setParam(p, (e.target as HTMLInputElement).value.trim())}
				/>
			{:else if p.kind === 'number'}
				<div class="ui-stepper">
					<button
						type="button"
						aria-label="减少"
						use:noNodeDrag
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
						placeholder={String(p.def ?? '')}
						use:noNodeDrag
						onchange={(e) => setParam(p, (e.target as HTMLInputElement).value)}
					/>
					<button
						type="button"
						aria-label="增加"
						use:noNodeDrag
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
					placeholder={String(p.def ?? '')}
					use:noNodeDrag
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
