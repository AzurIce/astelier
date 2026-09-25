// 确认对话框：替换原生 confirm()。用法：
//   const ok = await confirmDialog({ title, message, confirmText, danger })
// 调用方只管 await，UI 由 <ConfirmHost />（App 内挂载一次）呈现。
export interface ConfirmRequest {
	title: string
	/** 支持简单行内 HTML（调用方自行保证安全） */
	message?: string
	confirmText?: string
	cancelText?: string
	danger?: boolean
}

export interface PendingRequest extends ConfirmRequest {
	resolve: (v: boolean) => void
}

/** 导出可跟踪的状态壳（对象壳才能被组件模板的 $effect 依赖） */
export const confirmState = $state<{ req: PendingRequest | null }>({ req: null })

export function confirmDialog(req: ConfirmRequest): Promise<boolean> {
	return new Promise((resolve) => {
		// 已有对话框时排队会静默吞掉旧 Promise，先兜底取消上一个
		confirmState.req?.resolve(false)
		confirmState.req = { ...req, resolve }
	})
}

export function resolveConfirm(value: boolean) {
	const req = confirmState.req
	confirmState.req = null
	req?.resolve(value)
}
