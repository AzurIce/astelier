//! 批次快照弹窗：完整回看执行时刻的模板 / 输入 / 最终请求，
//! 并提供 原样重放 / 恢复模板 / 复制为新输入 三个动作。

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
    let Some(request) = run.request.clone() else {
        return rsx! {
            Modal {
                title: "旧版批次".to_string(),
                subtitle: None,
                wide: false,
                onclose: move |_| state.snapshot_run.set(None),
                div { class: "snap-legacy",
                    p { "该批次创建于快照机制之前，只保存了渲染后的 prompt 与参数，无法回看当时的模板 / 输入 / 图片。" }
                    p { class: "hint", "重跑将按当前配方与输入执行。" }
                }
            }
        };
    };

    let mut snapshot_run = state.snapshot_run;
    let close = move |_| snapshot_run.set(None);

    // ---- 动作 ----
    let rid_rerun = run.id.clone();
    let do_rerun = move |_| {
        let rid = rid_rerun.clone();
        let mut state = state;
        spawn(async move {
            match rerun_run(rid).await {
                Ok(new_run) => {
                    state.runs.with_mut(|v| v.insert(0, new_run));
                    state.toast("已按快照重放", "ok");
                }
                Err(e) => state.toast(format!("重跑失败：{e}"), "error"),
            }
        });
        // 不关弹窗：可以看到新批次出现在列表里
    };

    let rid_fork = run.id.clone();
    let do_fork = move |_| {
        let rid = rid_fork.clone();
        let mut state = state;
        spawn(async move {
            match new_input_from_run(rid).await {
                Ok(input) => {
                    let rid_recipe = input.recipe_id.clone();
                    let rid_input = input.id.clone();
                    if let Ok(list) = list_inputs(rid_recipe.clone()).await {
                        state.recipe_inputs.set(list);
                    }
                    state.select_input(&rid_recipe, &rid_input);
                    state.snapshot_run.set(None);
                    state.toast("已复制为新输入", "ok");
                }
                Err(e) => state.toast(format!("复制失败：{e}"), "error"),
            }
        });
    };

    let rid_restore = run.recipe_id.clone();
    let template_for_restore = request.template.clone();
    let do_restore = move |_| {
        let rid = rid_restore.clone();
        let t = template_for_restore.clone();
        let mut state = state;
        if let Some(mut recipe) = state.recipes().into_iter().find(|r| r.id == rid) {
            recipe.provider_id = t.provider_id;
            recipe.model_id = t.model_id;
            recipe.prompt_template = t.prompt_template;
            recipe.params = t.params;
            recipe.refs = t.refs;
            recipe.mask = t.mask;
            state.restore_template.set(Some(recipe));
            state.snapshot_run.set(None);
            state.toast("已载入模板快照（草稿），保存后成为新版本", "ok");
        }
    };

    let t = &request.template;
    let i = &request.input;
    let r = &request.resolved;
    let prompt_for_copy = request.resolved.prompt.clone();

    rsx! {
        Modal {
            title: "批次快照",
            subtitle: Some(format!(
                "模板 v{} · 输入 v{} · {} · {}",
                t.version, i.version, run.model_id, relative_time(run.created_at, now_ms())
            )),
            wide: true,
            onclose: close,

            div { class: "snapshot-body",
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

                // ---- 最终请求 ----
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
                    }                    if !r.images.is_empty() {
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

            div { class: "snap-foot",
                button {
                    class: "btn primary",
                    onclick: do_rerun,
                    crate::ui::icons::IconRefresh { size: 13 }
                    "原样重跑"
                }
                button {
                    class: "btn",
                    onclick: do_restore,
                    crate::ui::icons::IconSwap { size: 13 }
                    "恢复模板为此快照"
                }
                button {
                    class: "btn",
                    onclick: do_fork,
                    crate::ui::icons::IconCopy { size: 13 }
                    "复制为新输入"
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
