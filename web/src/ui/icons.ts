// 线性图标集（手写 SVG path，24×24 viewBox，stroke = currentColor）。
// 统一由 components/Icon.svelte 渲染：{@html} 注入到 <svg> 内。
// 约定：只用 stroke 风格（少量实心点除外），保持视觉一致。

export const ICONS = {
	// 品牌：圆角方 + 星芒（生成的火花）
	logo: '<path d="M12 3.2 14.6 9l5.8 2.6L14.6 14 12 19.8 9.4 14 3.6 11.6 9.4 9z" /><circle cx="18.4" cy="18.4" r="1.6" /><circle cx="5.6" cy="18.8" r="1.1" />',

	// 节点类型
	model: '<rect x="4.5" y="4.5" width="15" height="15" rx="3" /><rect x="9" y="9" width="6" height="6" rx="1" /><path d="M9 2.5v2M15 2.5v2M9 19.5v2M15 19.5v2M2.5 9h2M2.5 15h2M19.5 9h2M19.5 15h2" />',
	prompt: '<path d="M4 6.5V4.5h16v2M9.5 19.5h5M12 4.5v15" />',
	image: '<rect x="3" y="3" width="18" height="18" rx="2.5" /><circle cx="9" cy="9" r="1.8" /><path d="m20.5 15.5-3.6-3.6a2 2 0 0 0-2.8 0L5.5 21" />',
	generate:
		'<path d="M12 3.2 13.9 8.3 19 10.2l-5.1 1.9L12 17.2l-1.9-5.1L5 10.2l5.1-1.9z" /><path d="M18.8 14.6l.8 1.9 1.9.8-1.9.8-.8 1.9-.8-1.9-1.9-.8 1.9-.8z" />',
	preview: '<rect x="2.5" y="3.5" width="19" height="13" rx="2.5" /><path d="M8.5 20.5h7M12 16.5v4" />',

	// 侧栏 / 文件
	folder: '<path d="M20.5 19.5a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-8.1a2 2 0 0 1-1.7-.9l-.9-1.3A2 2 0 0 0 8.1 4H4a2 2 0 0 0-2 2v11.5a2 2 0 0 0 2 2z" />',
	graph: '<rect x="3" y="3" width="7.5" height="7.5" rx="1.8" /><rect x="13.5" y="3" width="7.5" height="7.5" rx="1.8" /><rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1.8" /><rect x="3" y="13.5" width="7.5" height="7.5" rx="1.8" />',

	// 操作
	plus: '<path d="M5 12h14M12 5v14" />',
	more: '<circle cx="5" cy="12" r="1" /><circle cx="12" cy="12" r="1" /><circle cx="19" cy="12" r="1" />',
	sidebarCollapse: '<rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16m7-12-4 4 4 4" />',
	sidebarExpand: '<rect x="3" y="4" width="18" height="16" rx="2" /><path d="M9 4v16m4-12 4 4-4 4" />',
	grip: '<circle cx="8" cy="5" r="1" /><circle cx="16" cy="5" r="1" /><circle cx="8" cy="12" r="1" /><circle cx="16" cy="12" r="1" /><circle cx="8" cy="19" r="1" /><circle cx="16" cy="19" r="1" />',
	play: '<path d="M7 4.5v15l12-7.5z" />',
	stop: '<rect x="6" y="6" width="12" height="12" rx="2" />',
	trash: '<path d="M3.5 6.5h17M9 6.5V4.8A1.8 1.8 0 0 1 10.8 3h2.4A1.8 1.8 0 0 1 15 4.8v1.7M18.5 6.5 17.6 19a2 2 0 0 1-2 1.9H8.4a2 2 0 0 1-2-1.9L5.5 6.5M10 10.5v6M14 10.5v6" />',
	search: '<circle cx="11" cy="11" r="7.5" /><path d="m21 21-4.4-4.4" />',
	sun: '<circle cx="12" cy="12" r="4" /><path d="M12 2.5v2M12 19.5v2M4.6 4.6l1.4 1.4M18 18l1.4 1.4M2.5 12h2M19.5 12h2M4.6 19.4 6 18M18 6l1.4-1.4" />',
	moon: '<path d="M12.5 3a7 7 0 0 0 9.5 9.5A9.5 9.5 0 1 1 12.5 3z" />',
	zoomIn: '<circle cx="11" cy="11" r="7.5" /><path d="m21 21-4.4-4.4M11 8v6M8 11h6" />',
	zoomOut: '<circle cx="11" cy="11" r="7.5" /><path d="m21 21-4.4-4.4M8 11h6" />',
	fit: '<path d="M8 3.5H5.5a2 2 0 0 0-2 2V8M21 8V5.5a2 2 0 0 0-2-2H16M16 20.5h2.5a2 2 0 0 0 2-2V16M3.5 16v2.5a2 2 0 0 0 2 2H8" />',
	chevronDown: '<path d="m6 9.5 6 6 6-6" />',
	chevronRight: '<path d="m9.5 6 6 6-6 6" />',
	x: '<path d="M18 6 6 18M6 6l12 12" />',
	check: '<path d="M20 6.5 9.5 17 4 11.5" />',
	alert: '<path d="M21.2 17.9l-7.6-13.2a2 2 0 0 0-3.5 0l-7.5 13.2A2 2 0 0 0 4.4 21h15.2a2 2 0 0 0 1.6-3.1zM12 9.5v4M12 17h.01" />',
	info: '<circle cx="12" cy="12" r="9.5" /><path d="M12 11v5M12 8h.01" />',
	spinner: '<path d="M21 12a9 9 0 1 1-6.2-8.6" />',
	upload: '<path d="M21 15.5V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-2.5M17 8.5 12 3.5 7 8.5M12 3.5V16" />',
	download: '<path d="M21 15.5V18a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-2.5M7 10.5l5 5 5-5M12 3.5V16" />',
	pencil: '<path d="M17.5 3.5a2.1 2.1 0 0 1 3 3L7.5 19.5 2.5 21l1.5-5z" />',
	settings:
		'<path d="M12.2 2.5h-.4a2 2 0 0 0-2 2v.2a2 2 0 0 1-1 1.7l-.4.3a2 2 0 0 1-2 0l-.2-.1a2 2 0 0 0-2.7.7l-.2.4a2 2 0 0 0 .7 2.7l.2.1a2 2 0 0 1 1 1.7v.5a2 2 0 0 1-1 1.8l-.2.1a2 2 0 0 0-.7 2.7l.2.4a2 2 0 0 0 2.7.7l.2-.1a2 2 0 0 1 2 0l.4.3a2 2 0 0 1 1 1.7v.2a2 2 0 0 0 2 2h.4a2 2 0 0 0 2-2v-.2a2 2 0 0 1 1-1.7l.4-.3a2 2 0 0 1 2 0l.2.1a2 2 0 0 0 2.7-.7l.2-.4a2 2 0 0 0-.7-2.7l-.2-.1a2 2 0 0 1-1-1.8v-.5a2 2 0 0 1 1-1.8l.2-.1a2 2 0 0 0 .7-2.7l-.2-.4a2 2 0 0 0-2.7-.7l-.2.1a2 2 0 0 1-2 0l-.4-.3a2 2 0 0 1-1-1.7v-.2a2 2 0 0 0-2-2z" /><circle cx="12" cy="12" r="2.8" />',
	clock: '<circle cx="12" cy="12" r="9.5" /><path d="M12 7.5V12l3 2" />',
	layers: '<path d="M12.8 2.4a2 2 0 0 0-1.6 0L2.7 6.3a1 1 0 0 0 0 1.8l8.5 3.9a2 2 0 0 0 1.6 0l8.5-3.9a1 1 0 0 0 0-1.8z" /><path d="m2.7 12.4 8.5 3.9a2 2 0 0 0 1.6 0l8.5-3.9M2.7 17.4l8.5 3.9a2 2 0 0 0 1.6 0l8.5-3.9" />',

	// 库浏览器（目录树 / 浏览）
	folderOpen:
		'<path d="M4 19.5a2 2 0 0 1-2-2V9a2 2 0 0 1 2-2h4.1a2 2 0 0 0 1.7-.9l.9-1.3A2 2 0 0 1 12.4 4H20a2 2 0 0 1 2 2v4" /><path d="M4.5 19.5 7.2 11a2 2 0 0 1 1.9-1.4h15.4a1.5 1.5 0 0 1 1.4 2l-2.6 8.3a2 2 0 0 1-1.9 1.6z" />',
	folderPlus:
		'<path d="M20.5 19.5a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-8.1a2 2 0 0 1-1.7-.9l-.9-1.3A2 2 0 0 0 8.1 4H4a2 2 0 0 0-2 2v11.5a2 2 0 0 0 2 2z" /><path d="M17 12v6M14 15h6" />',
	move: '<path d="M5 9.5 9.5 5m0 0H6m3.5 0V8.5" /><path d="M19 14.5 14.5 19m0 0H18m-3.5 0V15.5" /><path d="M9.5 5H19v9.5" /><path d="M14.5 19H5V9.5" />',
	maximize: '<path d="M4 9V5.5A1.5 1.5 0 0 1 5.5 4H9M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15" />',
	restore: '<path d="M9 4H5.5A1.5 1.5 0 0 0 4 5.5V9M15 4h3.5A1.5 1.5 0 0 1 20 5.5V9M20 15v3.5a1.5 1.5 0 0 1-1.5 1.5H15M9 20H5.5A1.5 1.5 0 0 1 4 18.5V15" />',
	refresh: '<path d="M20.5 12a8.5 8.5 0 1 1-2.6-6.1" /><path d="M20.5 4v4.5H16" />',
	minus: '<path d="M5 12h14" />',
	list: '<path d="M8.5 6.5H21M8.5 12H21M8.5 17.5H21M3.5 6.5h.01M3.5 12h.01M3.5 17.5h.01" />',
} as const

export type IconName = keyof typeof ICONS
