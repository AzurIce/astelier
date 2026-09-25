// 轻通知：替换 alert()。任意模块 import { toast } 即可推送。
import type { IconName } from '../design/icons'

export type ToastKind = 'info' | 'ok' | 'err'

export interface Toast {
	id: number
	kind: ToastKind
	title: string
	msg?: string
	timeout: number
}

let seq = 0

export const toasts = $state<Toast[]>([])

function dismiss(id: number) {
	const i = toasts.findIndex((t) => t.id === id)
	if (i >= 0) toasts.splice(i, 1)
}

export interface ToastInput {
	kind?: ToastKind
	title: string
	msg?: string
	/** 停留 ms；0 = 不自动消失 */
	timeout?: number
}

export function toast(input: ToastInput): number {
	const id = ++seq
	toasts.push({
		id,
		kind: input.kind ?? 'info',
		title: input.title,
		msg: input.msg,
		timeout: input.timeout ?? (input.kind === 'err' ? 12000 : 4200),
	})
	if (toasts[toasts.length - 1].timeout > 0) {
		setTimeout(() => dismiss(id), input.timeout ?? (input.kind === 'err' ? 12000 : 4200))
	}
	return id
}

export function dismissToast(id: number) {
	dismiss(id)
}

export const TOAST_ICONS: Record<ToastKind, IconName> = {
	info: 'info',
	ok: 'check',
	err: 'alert',
}
