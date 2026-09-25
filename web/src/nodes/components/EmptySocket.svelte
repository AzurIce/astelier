<script lang="ts">
	// classic preset 的 socket 占位组件：不渲染任何可见元素。
	// 端口圆点由节点组件里的 .ui-socket 自绘；这里必须挂载一个组件，
	// 否则 renderer 不会发出 rendered 信号，DOMSocketPosition 拿不到
	// 端口元素，连线端点定位会失效（曲线起点飘走）。
</script>

<span class="ui-empty-socket"></span>

<style>
	/* 不占布局、不可见：端口视觉完全由外层 .ui-socket 承担 */
	.ui-empty-socket {
		display: none;
	}
</style>
