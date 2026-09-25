<script lang="ts">
	import Icon from '../../components/Icon.svelte'
	import type { IconName } from '../../design/icons'
	import type { Snippet } from 'svelte'
	import type { NodeType } from '../classes'

	// 节点外框：标题栏（类型图标 + 名称 + 描述 + 删除）+ 输入区 / 主体 / 输出区。
	// 保持 data-node-id（右键选中依赖）与 .ui-node/.selected 结构不变。
	let {
		nodeId,
		type,
		icon,
		name,
		desc,
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
		desc: string
		selected?: boolean
		busy?: boolean
		ondelete: () => void
		inputs?: Snippet
		body?: Snippet
		outputs?: Snippet
	} = $props()
</script>

<div class="ui-node" class:selected class:busy data-node-id={nodeId} data-node-type={type}>
	<header>
		<span
			class="type-icon"
			style:color={`var(--ui-sock-${type === 'generate' || type === 'preview' ? 'image' : type})`}
			style:background={`color-mix(in srgb, var(--ui-sock-${type === 'generate' || type === 'preview' ? 'image' : type}) 16%, transparent)`}
		>
			<Icon name={icon} size={13} />
		</span>
		<span class="titles">
			<strong>{name}</strong>
			{#if desc}<small>{desc}</small>{/if}
		</span>
		<span class="head-actions">
			<button type="button" title="删除节点" aria-label="删除节点" onclick={ondelete}>
				<Icon name="trash" size={13} />
			</button>
		</span>
	</header>
	{#if inputs}
		<div class="node-ports in">
			{@render inputs()}
		</div>
	{/if}
	{#if body}
		<div class="node-body">
			{@render body()}
		</div>
	{/if}
	{#if outputs}
		<div class="node-ports out">
			{@render outputs()}
		</div>
	{/if}
</div>
