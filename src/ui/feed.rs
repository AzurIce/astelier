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
        .effective_params()
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
    let rid_fork = run.id.clone();
    let rid_snapshot = run.id.clone();
    let prompt_preview: String = {
        let t = run.display_prompt().replace('\n', " ");
        if t.chars().count() > 120 {
            format!("{}…", t.chars().take(120).collect::<String>())
        } else {
            t
        }
    };
    let img_count = run.image_count();
    let has_snapshot = run.request.is_some();
    // 配方当前版本 ≠ 批次版本 → 提示模板已演进
    let current_recipe_version = state
        .recipes()
        .into_iter()
        .find(|r| r.id == run.recipe_id)
        .map(|r| r.version);

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
                    if let Some(cv) = current_recipe_version {
                        if cv != run.recipe_version {
                            span {
                                class: "dot-stale",
                                title: "配方已更新至 v{cv}，此批次基于 v{run.recipe_version} 的快照",
                            }
                        }
                    }
                    span { class: "badge version-chip", "模板v{run.recipe_version}" }
                    if run.input_version > 0 {
                        span { class: "badge version-chip", "输入v{run.input_version}" }
                    }
                    if run.rerun_of.is_some() {
                        span { class: "badge rerun-chip", title: "对原批次的快照重放", "重放" }
                    }
                    if !has_snapshot {
                        span { class: "badge legacy-chip", title: "旧版批次，仅保存了渲染结果", "旧版" }
                    }
                    if img_count > 0 {
                        span { class: "run-refs", "{img_count} 图" }
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
                        title: if has_snapshot { "原样重跑（按批次快照）" } else { "重跑（旧版批次将按当前配方/输入）" },
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
                        disabled: !has_snapshot,
                        title: if has_snapshot { "查看批次快照（当时的模板 / 输入 / 最终请求）" } else { "旧版批次没有快照" },
                        onclick: move |_| {
                            state.snapshot_run.set(Some(rid_snapshot.clone()));
                        },
                        crate::ui::icons::IconEye { size: 14 }
                    }
                    button {
                        class: "icon-btn",
                        title: if has_snapshot { "复制为新输入" } else { "旧版批次无法复制输入" },
                        disabled: !has_snapshot,
                        onclick: move |_| {
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
                                        state.toast("已复制为新输入", "ok");
                                    }
                                    Err(e) => state.toast(format!("复制失败：{e}"), "error"),
                                }
                            });
                        },
                        crate::ui::icons::IconCopy { size: 14 }
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
