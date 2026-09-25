// 主题切换：data-theme 挂在 <html> 上，值由 index.html 头部脚本先行写入。
export type Theme = 'light' | 'dark'

export function currentTheme(): Theme {
	return document.documentElement.dataset.theme === 'light' ? 'light' : 'dark'
}

export function toggleTheme(): Theme {
	const next: Theme = currentTheme() === 'dark' ? 'light' : 'dark'
	document.documentElement.dataset.theme = next
	localStorage.setItem('atelier-theme', next)
	return next
}
