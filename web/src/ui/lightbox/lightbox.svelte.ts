// 大图预览：点击节点内图片打开，Esc / 点击关闭
export const lightbox = $state<{ url: string | null }>({ url: null })

export function openLightbox(url: string): void {
	lightbox.url = url
}
export function closeLightbox(): void {
	lightbox.url = null
}
