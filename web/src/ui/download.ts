export function downloadFile(file: File): void {
	const url = URL.createObjectURL(file)
	const anchor = document.createElement('a')
	anchor.href = url
	anchor.download = file.name
	anchor.click()
	setTimeout(() => URL.revokeObjectURL(url), 10_000)
}
