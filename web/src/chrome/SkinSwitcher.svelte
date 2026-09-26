<script lang="ts">
	import Popover from '../components/Popover.svelte'
	import Icon from '../components/Icon.svelte'
	import { design } from '../design/store.svelte'
	import { SKINS } from '../design/tokens'
	import type { TokenSet } from '../design/tokens'

	// 皮肤切换：按钮上带当前皮肤三色板 + 名称；浮层三选一
	let open = $state(false)
	let anchor = $state({ x: 0, y: 0 })

	function openFrom(e: MouseEvent) {
		const r = (e.target as HTMLElement).getBoundingClientRect()
		anchor = { x: r.right, y: r.bottom + 6 }
		open = true
	}

	/** 预览四格：暗底 / 面板 / 卡片 / 强调（取当前明暗模式的该皮肤令牌） */
	function cells(t: TokenSet): string[] {
		return [t.bg, t.panel, t.card, t.accent]
	}
</script>

<button type="button" class="skin-entry" onclick={openFrom} title="切换外观">
	<span class="dots">
		{#each design.skin.swatch as c, i (i)}
			<i style:background={c}></i>
		{/each}
	</span>
	<span class="name">{design.skin.name}</span>
	<Icon name="chevronDown" size={12} />
</button>

<Popover bind:open {anchor} panelClass="skin-pop" onclose={() => (open = false)}>
	{#each SKINS as s (s.id)}
		{@const t = s[design.mode]}
		<button
			type="button"
			class="skin-opt"
			class:active={s.id === design.skin.id}
			onclick={() => design.setSkin(s.id)}
		>
			<span class="preview">
				{#each cells(t) as c, i (i)}
					<i style:background={c}></i>
				{/each}
			</span>
			<span class="meta">
				<strong>{s.name}</strong>
				<small>{s.blurb}</small>
			</span>
			{#if s.id === design.skin.id}<Icon name="check" size={14} />{/if}
		</button>
	{/each}
</Popover>
