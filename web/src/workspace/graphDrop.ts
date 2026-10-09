export interface GraphImportDestination { backendId: string; groupId: string | null }
export interface GraphDropPreview extends GraphImportDestination {
	fileLabel: string
	backendName: string
	folderName: string
	allowed: boolean
	sidebar: boolean
	x: number
	y: number
	width: number
}

export function graphDropDestination(target: EventTarget | null, activeBackendId: string): GraphImportDestination {
	const element = target instanceof Element ? target : null
	const root = element?.closest<HTMLElement>('[data-backend-id]')
	const folder = root ? element?.closest<HTMLElement>('[data-row-kind="dir"]') : null
	return { backendId: root?.dataset.backendId ?? activeBackendId, groupId: folder?.dataset.rowId ?? null }
}

/** OS drags hide filenames until drop; unknown file types get an .astelier hint. */
export function isGraphArchiveDrag(transfer: DataTransfer): boolean {
	if (!transfer.types.includes('Files')) return false
	const files = [...transfer.files]
	if (files.length) return files.some((file) => /\.astelier$/i.test(file.name))
	return [...transfer.items].some((item) => item.kind === 'file' && ['', 'application/zip', 'application/x-zip-compressed', 'application/octet-stream'].includes(item.type))
}
