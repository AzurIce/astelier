<script lang="ts">
	import Icon from '../../../ui/Icon.svelte'
	import type { IconName } from '../../../ui/icons'
	import type { Snippet } from 'svelte'
	import type { NodeType } from '../../../workspace/types'

	// 节点外框：标题栏 + 左右端口列（输入左 / 输出右）+ 中列内容。
	// 端口圆点骑在节点边框上（负半边），连线端点即圆点中心。
	// 保持 data-node-id（右键选中依赖）与 .ui-node/.selected 结构。
	let {
		nodeId,
		type,
		icon,
		name,
		desc = '',
		title,
		selected = false,
		busy = false,
		ondelete,
		inputs,
		body,
		outputs,
	}: {
		nodeId: string
		type: NodeType
		icon: IconName
		name: string
		desc?: string
		/** desc 的动态版本（优先于 desc） */
		title?: string
		selected?: boolean
		busy?: boolean
		ondelete: () => void
		inputs?: Snippet
		body?: Snippet
		outputs?: Snippet
	} = $props()	// 标题图标的强调色 token；store 无专属端口色，用 accent
	const tone = $derived(
		type === 'model' || type === 'prompt'
			? `sock-${type}`
			: type === 'image' || type === 'generate' || type === 'preview'
				? 'sock-image'
				: 'accent',
	)
</script>

<div class="ui-node" class:selected class:busy data-node-id={nodeId} data-node-type={type}>
	<header>
		<span
			class="type-icon"
			style:color={`var(--ui-${tone})`}
			style:background={`color-mix(in srgb, var(--ui-${tone}) 16%, transparent)`}
		>
			<Icon name={icon} size={13} />
		</span>
		<span class="titles">
			<strong>{name}</strong>
			{#if title || desc}<small>{title || desc}</small>{/if}
		</span>
		<span class="head-actions">
			<button type="button" title="删除节点" aria-label="删除节点" onclick={ondelete}>
				<Icon name="trash" size={13} />
			</button>
		</span>
	</header>

	<div class="node-main">
		{#if inputs}
			<div class="ports left">
				{@render inputs()}
			</div>
		{/if}
		{#if body}
			<div class="node-body">
				{@render body()}
			</div>
		{/if}
		{#if outputs}
			<div class="ports right">
				{@render outputs()}
			</div>
		{/if}
	</div>
</div>
