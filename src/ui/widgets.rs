//! 通用 UI 小组件：Modal / Dropdown / Segmented / Stepper / Toast / RefsStrip。

use crate::app::AppState;
use crate::model::AssetRef;
use dioxus::prelude::*;

// ---------- RefsStrip ----------

/// 资产缩略图条 + 上传/移除。上传立即落盘成资产，归属由 on_added 回调决定
/// （配方固定图 / 输入额外参考图共用）。
#[component]
pub fn RefsStrip(
    state: AppState,
    refs: Vec<AssetRef>,
    on_added: EventHandler<AssetRef>,
    on_removed: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "ref-strip",
            div { class: "ref-thumbs",
                for r in refs.iter().cloned() {
                    div { key: "{r.id}", class: "ref-thumb",
                        img { src: "{r.url()}", loading: "lazy" }
                        button {
                            class: "ref-thumb-remove",
                            title: "移除",
                            onclick: move |_| on_removed(r.id.clone()),
                            crate::ui::icons::IconX { size: 10 }
                        }
                    }
                }
                label { class: "ref-add",
                    title: "上传参考图",
                    crate::ui::icons::IconImage { size: 15 }
                    span { "参考图" }
                    input {
                        r#type: "file",
                        accept: "image/*",
                        multiple: true,
                        style: "display:none",
                        onchange: move |e| {
                            let files = e.files();
                            spawn(async move {
                                for file in files {
                                    let filename = file.name();
                                    match file.read_bytes().await {
                                        Ok(bytes) => match crate::api::upload_asset(bytes.to_vec(), filename).await {
                                            Ok(asset) => on_added(asset),
                                            Err(err) => state.toast(format!("上传失败：{err}"), "error"),
                                        },
                                        Err(e) => state.toast(format!("读取文件失败：{e}"), "error"),
                                    }
                                }
                            });
                        },
                    }
                }
            }
        }
    }
}

// ---------- Modal ----------

#[component]
pub fn Modal(
    title: String,
    subtitle: Option<String>,
    wide: bool,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "modal-backdrop",
            onclick: move |_| onclose(()),
            div {
                class: if wide { "modal wide" } else { "modal" },
                onclick: move |e| e.stop_propagation(),
                header {
                    class: "modal-head",
                    div {
                        h2 { "{title}" }
                        if let Some(sub) = &subtitle {
                            p { class: "modal-sub", "{sub}" }
                        }
                    }
                    button {
                        class: "icon-btn",
                        onclick: move |_| onclose(()),
                        "aria-label": "关闭",
                        crate::ui::icons::IconX { size: 16 }
                    }
                }
                div { class: "modal-body", {children} }
            }
        }
    }
}

// ---------- Dropdown ----------

#[derive(Clone, PartialEq)]
pub struct DropdownItem {
    pub value: String,
    pub label: String,
    /// 菜单里的次级说明
    pub caption: Option<String>,
    /// 选中行右侧的小徽标
    pub badges: Vec<(&'static str, &'static str)>,
}

/// 简约下拉：按钮 + 浮层菜单（backdrop 关闭），非原生 select。
#[component]
pub fn Dropdown(
    label: String,
    caption: Option<String>,
    items: Vec<DropdownItem>,
    value: String,
    onpick: EventHandler<String>,
    class: Option<String>,
    disabled: bool,
    /// 在底部使用时向上弹出菜单
    drop_up: bool,
) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        div {
            class: if disabled { "dropdown disabled {class.unwrap_or_default()}" } else { "dropdown {class.unwrap_or_default()}" },
            button {
                class: "dropdown-btn",
                disabled,
                onclick: move |_| open.toggle(),
                span {
                    class: "dropdown-label",
                    title: "{label}",
                    "{label}"
                }
                if let Some(cap) = &caption {
                    span { class: "dropdown-caption", "{cap}" }
                }
                span { class: "dropdown-caret", crate::ui::icons::IconChevronDown { size: 14 } }
            }
            if open() {
                div {
                    class: "dropdown-backdrop",
                    onclick: move |_| open.set(false),
                }
                div { class: if drop_up { "dropdown-menu up" } else { "dropdown-menu" },
                    for item in items.iter().cloned() {
                        button {
                            class: if item.value == value { "dropdown-item selected" } else { "dropdown-item" },
                            onclick: move |_| {
                                open.set(false);
                                onpick(item.value.clone());
                            },
                            span { class: "dropdown-item-main",
                                span { class: "dropdown-item-label", "{item.label}" }
                                if let Some(cap) = &item.caption {
                                    span { class: "dropdown-item-caption", "{cap}" }
                                }
                            }
                            if !item.badges.is_empty() {
                                span { class: "dropdown-item-badges",
                                    for (text, kind) in item.badges.iter() {
                                        span { class: "badge badge-{kind}", "{text}" }
                                    }
                                }
                            }
                            if item.value == value {
                                span { class: "dropdown-check", crate::ui::icons::IconCheck { size: 13 } }
                            }
                        }
                    }
                }
            }
        }
    }
}

// ---------- Segmented ----------

#[derive(Clone, PartialEq)]
pub struct Segment {
    pub value: String,
    pub label: String,
    pub title: Option<String>,
}

#[component]
pub fn Segmented(
    segments: Vec<Segment>,
    value: String,
    onpick: EventHandler<String>,
    compact: bool,
) -> Element {
    rsx! {
        div {
            class: if compact { "segmented compact" } else { "segmented" },
            for seg in segments.iter().cloned() {
                button {
                    class: if seg.value == value {
                        "segment active"
                    } else {
                        "segment"
                    },
                    title: seg.title.clone().unwrap_or_default(),
                    onclick: move |_| onpick(seg.value.clone()),
                    "{seg.label}"
                }
            }
        }
    }
}

// ---------- Stepper ----------

#[component]
pub fn Stepper(
    value: Option<f64>,
    min: f64,
    max: f64,
    step: f64,
    onchange: EventHandler<Option<f64>>,
) -> Element {
    let shown = value.map(|v| if step < 1.0 { format!("{v:.1}") } else if v.fract() == 0.0 { format!("{}", v as i64) } else { format!("{v}") });
    let shown_text = shown.clone().unwrap_or_else(|| "–".into());
    let title_text = shown
        .clone()
        .unwrap_or_else(|| "未设置（点 + 从最小值开始）".to_string());
    rsx! {
        div {
            class: if value.is_none() { "stepper unset" } else { "stepper" },
            button {
                class: "stepper-btn",
                tabindex: "-1",
                title: "减小",
                onclick: move |_| {
                    if let Some(v) = value {
                        onchange(Some((v - step).max(min)));
                    }
                },
                "−"
            }
            button {
                class: "stepper-value",
                title: "{title_text}",
                "{shown_text}"
            }
            button {
                class: "stepper-btn",
                tabindex: "-1",
                title: "增大",
                onclick: move |_| {
                    let base = value.unwrap_or(min);
                    onchange(Some((base + step).min(max)));
                },
                "+"
            }
            if value.is_some() {
                button {
                    class: "stepper-clear",
                    tabindex: "-1",
                    title: "恢复默认（不发送）",
                    onclick: move |_| onchange(None),
                    crate::ui::icons::IconX { size: 11 }
                }
            }
        }
    }
}

// ---------- Toast ----------

#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub text: String,
    pub kind: &'static str,
}

#[component]
pub fn Toasts(toasts: Signal<Vec<Toast>>) -> Element {
    rsx! {
        if !toasts().is_empty() {
            div { class: "toasts",
                for t in toasts().iter() {
                    div { key: "{t.id}", class: "toast toast-{t.kind}", "{t.text}" }
                }
            }
        }
    }
}
