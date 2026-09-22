//! Run（批次）卡片与全局批次视图。

use crate::api::*;
use crate::app::AppState;
use crate::model::fmt::{relative_time, usage_text};
use crate::model::{Run, RunStatus};
use crate::util::now_ms;
use dioxus::prelude::*;

/// 全局批次视图（未选中配方时）
#[component]
pub fn Feed(state: AppState) -> Element {
    let runs = state.runs();
    let now = now_ms();

    if runs.is_empty() {
        return rsx! {
            div { class: "feed empty-feed",
                div { class: "empty-state",
                    div { class: "empty-glyph", crate::ui::icons::IconSparkles { size: 28 } }
                    h3 { "从一次生成开始" }
                    p { "在左侧新建配方，写好 Prompt 模板，然后创建输入开始出图。" }
                }
            }
        };
    }

    rsx! {
        div { class: "feed",
            for run in runs.iter() {
                RunCard { key: "{run.id}", state, run: run.clone(), now }
            }
        }
    }
}

#[component]
pub fn RunCard(state: AppState, run: Run, now: u64) -> Element {
    let runs_sig = state.runs;
    let status = run.status;
    let n_expected = run
        .params
        .get("n")
        .and_then(|v| match v {
            crate::model::ParamValue::Number(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1)
        .clamp(1, 10);
    let dur_s = run
        .duration_ms
        .map(|ms| format!("{:.1}", ms as f64 / 1000.0));
    let rid_rerun = run.id.clone();
    let rid_del = run.id.clone();
    let prompt_preview: String = {
        let t = run.prompt.replace('\n', " ");
        if t.chars().count() > 120 {
            format!("{}…", t.chars().take(120).collect::<String>())
        } else {
            t
        }
    };

    rsx! {
        article {
            class: match status {
                RunStatus::Running => "run-card running",
                RunStatus::Done => "run-card",
                RunStatus::Error => "run-card error",
            },
            header { class: "run-head",
                div { class: "run-head-left",
                    span {
                        class: match status {
                            RunStatus::Running => "run-status spin",
                            RunStatus::Done => "run-status ok",
                            RunStatus::Error => "run-status err",
                        },
                        if status == RunStatus::Running {
                            crate::ui::icons::IconLoader { size: 13 }
                        } else if status == RunStatus::Error {
                            crate::ui::icons::IconAlert { size: 13 }
                        } else {
                            crate::ui::icons::IconCheck { size: 13 }
                        }
                    }
                    span { class: "mono run-model", "{run.model_id}" }
                    span { class: "badge run-mode", "{run.mode.label()}" }
                    span { class: "badge version-chip", "v{run.recipe_version}" }
                    if run.ref_count > 0 {
                        span { class: "run-refs", "{run.ref_count} 图" }
                    }
                    span { class: "run-time", "{relative_time(run.created_at, now)}" }
                    if let Some(ds) = &dur_s {
                        span { class: "run-dur", "{ds}s" }
                    }
                    if let Some(u) = &run.usage {
                        span { class: "run-usage", "{usage_text(u)}" }
                    }
                }
                div { class: "run-head-actions",
                    button {
                        class: "icon-btn",
                        title: "原样重跑（使用输入的当前值）",
                        onclick: move |_| {
                            let rid = rid_rerun.clone();
                            spawn(async move {
                                if let Err(e) = rerun_run(rid).await {
                                    state.toast(format!("重跑失败：{e}"), "error");
                                }
                            });
                        },
                        crate::ui::icons::IconRefresh { size: 14 }
                    }
                    button {
                        class: "icon-btn",
                        title: "删除批次",
                        onclick: move |_| {
                            let rid = rid_del.clone();
                            let mut runs_sig = runs_sig;
                            spawn(async move {
                                let _ = delete_run(rid.clone()).await;
                                runs_sig.write().retain(|r| r.id != rid);
                            });
                        },
                        crate::ui::icons::IconTrash { size: 14 }
                    }
                }
            }

            if !prompt_preview.is_empty() {
                div { class: "run-prompt", title: "渲染后的完整 Prompt", "{prompt_preview}" }
            }

            if let Some(err) = &run.error {
                div { class: "run-error", "{err}" }
            }

            div { class: "run-grid",
                for (i, img) in run.images.iter().enumerate() {
                    ImageTile { key: "{img.id}", state, asset: img.clone(), run_id: run.id.clone(), index: i }
                }
                if status == RunStatus::Running {
                    for i in run.images.len()..n_expected {
                        div { key: "ph-{i}", class: "tile placeholder",
                            div { class: "shimmer" }
                            span { class: "ph-hint", "生成中…" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ImageTile(state: AppState, asset: crate::model::AssetRef, run_id: String, index: usize) -> Element {
    let url = asset.url();
    let rid_img = run_id.clone();
    let rid_zoom = run_id.clone();

    rsx! {
        div { class: "tile",
            img {
                src: "{url}",
                loading: "lazy",
                onclick: move |_| state.lightbox.set(Some((rid_img.clone(), index))),
            }
            div { class: "tile-actions",
                button {
                    class: "icon-btn tile-btn",
                    title: "放大",
                    onclick: move |_| state.lightbox.set(Some((rid_zoom.clone(), index))),
                    crate::ui::icons::IconImage { size: 14 }
                }
                button {
                    class: "icon-btn tile-btn",
                    title: "下载",
                    onclick: move |_| {
                        // 浏览器下载交给 <a download>；这里保留放大为主操作
                        state.lightbox.set(Some((run_id.clone(), index)));
                    },
                    crate::ui::icons::IconDownload { size: 14 }
                }
            }
        }
    }
}
