//! 批次快照弹窗：回看执行时刻归档的内容。
//!
//! - 节点图批次：归档了最终请求（prompt / 参数 / 图片）
//! - 配方时代批次：归档了当时的模板 / 输入 / 最终请求
//! - 更早的旧批次：只有渲染结果

use crate::api::*;
use crate::app::AppState;
use crate::model::{fmt::relative_time, AssetRef, MaskOverride, ParamValue};
use crate::ui::widgets::Modal;
use crate::util::now_ms;
use dioxus::prelude::*;

#[component]
pub fn SnapshotModal(state: AppState, run_id: String) -> Element {
    let Some(run) = state.runs().into_iter().find(|r| r.id == run_id) else {
        return rsx! {};
    };
    let mut snapshot_run = state.snapshot_run;
    let close = move |_| snapshot_run.set(None);

    let can_rerun = run.request.is_some() || run.resolved.is_some();
    let rid_rerun = run.id.clone();
    let do_rerun = move |_| {
        let rid = rid_rerun.clone();
        let mut state = state;
        spawn(async move {
            match rerun_run(rid).await {
                Ok(new_run) => {
                    state.runs.with_mut(|v| v.insert(0, new_run));
                    state.toast("已按归档重放", "ok");
                }
                Err(e) => state.toast(format!("重跑失败：{e}"), "error"),
            }
        });
        // 不关弹窗：可以看到新批次出现在列表里
    };

    let source_sub = if let Some(t) = run.request.as_ref().map(|r| &r.template) {
        format!(
            "模板 v{} · 输入 v{} · {} · {}",
            t.version,
            run.request.as_ref().map(|r| r.input.version).unwrap_or(0),
            run.model_id,
            relative_time(run.created_at, now_ms())
        )
    } else {
        format!("{} · {}", run.model_id, relative_time(run.created_at, now_ms()))
    };

    rsx! {
        Modal {
            title: "批次归档",
            subtitle: Some(source_sub),
            wide: true,
            onclose: close,

            div { class: "snapshot-body",
                {match &run.request {
                    Some(request) => rsx! {
                        RecipeSections { request: request.clone() }
                    },
                    None => rsx! {},
                }}

                {match run.resolved.as_ref().or(run.request.as_ref().map(|r| &r.resolved)) {
                    Some(r) => rsx! {
                        ResolvedSection { resolved: r.clone() }
                    },
                    None => rsx! {
                        section { class: "snap-section",
                            p { "该批次创建于快照机制之前，只保存了渲染后的 prompt 与参数，没有可回看的归档。" }
                        }
                    },
                }}
            }

            div { class: "snap-foot",
                if can_rerun {
                    button {
                        class: "btn primary",
                        onclick: do_rerun,
                        crate::ui::icons::IconRefresh { size: 13 }
                        "原样重跑"
                    }
                }
            }
        }
    }
}

/// 配方时代快照（模板 + 输入）
#[component]
fn RecipeSections(request: crate::model::RunRequest) -> Element {
    let t = &request.template;
    let i = &request.input;
    rsx! {
        // ---- 模板快照 ----
        section { class: "snap-section",
            h3 { "模板快照" }
            div { class: "snap-kv",
                span { class: "snap-k", "模型" }
                span { class: "mono snap-v", "{t.model_id}" }
            }
            pre { class: "snap-pre", "{t.prompt_template}" }
            if !t.params.is_empty() {
                ParamTable { label: "参数", params: t.params.clone() }
            }
            if !t.refs.is_empty() {
                AssetRow { label: "固定参考图", assets: t.refs.clone() }
            }
            if let Some(m) = &t.mask {
                div { class: "snap-kv",
                    span { class: "snap-k", "Mask" }
                    img { class: "mask-preview", src: "{m.url()}" }
                }
            }
        }

        // ---- 输入快照 ----
        section { class: "snap-section",
            h3 { "输入快照" }
            if !i.variables.is_empty() {
                div { class: "snap-kv-wrap",
                    for (k, v) in i.variables.iter() {
                        div { class: "snap-kv",
                            span { class: "snap-k", "{{{k}}}" }
                            span { class: "snap-v", "{v}" }
                        }
                    }
                }
            }
            if !i.images.is_empty() {
                div { class: "snap-kv-wrap",
                    for (k, a) in i.images.iter() {
                        div { class: "snap-kv",
                            span { class: "snap-k", "{{img:{k}}}" }
                            img { class: "snap-thumb", src: "{a.url()}", loading: "lazy" }
                        }
                    }
                }
            }
            if !i.extra_refs.is_empty() {
                AssetRow { label: "额外参考图", assets: i.extra_refs.clone() }
            }
            if let Some(mo) = &i.mask_override {
                div { class: "snap-kv",
                    span { class: "snap-k", "Mask" }
                    {match mo {
                        MaskOverride::Off => rsx! { span { class: "snap-v", "不使用（覆盖配方）" } },
                        MaskOverride::Custom(a) => rsx! { img { class: "mask-preview", src: "{a.url()}" } },
                    }}
                }
            }
            if !i.param_overrides.is_empty() {
                ParamTable { label: "参数覆盖", params: i.param_overrides.clone() }
            }
        }
    }
}

/// 最终请求（两类批次共有）
#[component]
fn ResolvedSection(resolved: crate::model::ResolvedRequest) -> Element {
    let r = resolved;
    let prompt_for_copy = r.prompt.clone();
    rsx! {
        section { class: "snap-section snap-resolved",
            h3 { "最终请求" }
            div { class: "snap-kv",
                span { class: "snap-k", "模式" }
                span { class: "snap-v", "{r.mode.label()}" }
            }
            div { class: "snap-prompt-row",
                pre { class: "snap-pre", "{r.prompt}" }
                button {
                    class: "ghost-btn small",
                    title: "复制 prompt",
                    onclick: move |_| copy_to_clipboard(&prompt_for_copy),
                    crate::ui::icons::IconCopy { size: 12 }
                    "复制"
                }
            }
            if !r.params.is_empty() {
                ParamTable { label: "发送参数", params: r.params.clone() }
            }
            if !r.images.is_empty() {
                AssetRow { label: "发送图片（按顺序）", assets: r.images.clone() }
            }
            if let Some(m) = &r.mask {
                div { class: "snap-kv",
                    span { class: "snap-k", "Mask" }
                    img { class: "mask-preview", src: "{m.url()}" }
                }
            }
        }
    }
}

fn copy_to_clipboard(text: &str) {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', "\\n")
        .replace('\r', "");
    let _ = dioxus::document::eval(&format!(
        "navigator.clipboard.writeText('{escaped}')"
    ));
}

#[component]
fn ParamTable(label: String, params: std::collections::BTreeMap<String, ParamValue>) -> Element {
    rsx! {
        div { class: "snap-kv-wrap",
            span { class: "snap-k snap-k-head", "{label}" }
            div { class: "snap-param-table",
                for (k, v) in params.iter() {
                    div { key: "{k}", class: "snap-param-row",
                        span { class: "mono snap-param-key", "{k}" }
                        span { class: "snap-param-val", {param_text(v)} }
                    }
                }
            }
        }
    }
}

fn param_text(v: &ParamValue) -> String {
    match v {
        ParamValue::Unset => "—（不发送）".into(),
        ParamValue::Text(s) => s.clone(),
        ParamValue::Number(n) => format!("{n}"),
        ParamValue::Size(s) => s.clone(),
    }
}

#[component]
fn AssetRow(label: String, assets: Vec<AssetRef>) -> Element {
    rsx! {
        div { class: "snap-kv-wrap",
            span { class: "snap-k snap-k-head", "{label}" }
            div { class: "snap-assets",
                for (idx, a) in assets.iter().enumerate() {
                    div { key: "{a.id}", class: "snap-asset",
                        img { src: "{a.url()}", loading: "lazy" }
                        span { class: "seq-num", "{idx + 1}" }
                    }
                }
            }
        }
    }
}
