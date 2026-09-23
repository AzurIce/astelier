//! 配方页：模板编辑（可折叠）+ 输入准备 + 批次流，三层结构对应
//! 配方（模板）→ 输入（一次具体化）→ 批次（自包含快照）。
//!
//! 布局响应式：宽屏「编辑 | 批次」双栏独立滚动，窄屏纵向堆叠。

use crate::app::AppState;
use crate::model::{Recipe, Run};
use crate::util::now_ms;
use dioxus::prelude::*;

pub mod input_band;
pub mod template_band;

#[component]
pub fn RecipePage(state: AppState, recipe: Recipe) -> Element {
    rsx! {
        div { class: "stage recipe-page",
            div { class: "edit-col",
                template_band::TemplateBand { state, recipe: recipe.clone() }
                input_band::InputBand { state, recipe: recipe.clone() }
            }

            RunsBand { state, recipe }
        }
    }
}

/// 此配方的批次流；选中输入时只看它的批次。
#[component]
fn RunsBand(state: AppState, recipe: Recipe) -> Element {
    let runs_all = state.runs();
    let input_filter = state.selected_input_id();
    let runs: Vec<Run> = runs_all
        .iter()
        .filter(|r| {
            r.recipe_id == recipe.id
                && (input_filter.is_empty() || r.input_id.as_deref() == Some(input_filter.as_str()))
        })
        .cloned()
        .collect();

    let filter_empty = input_filter.is_empty();
    rsx! {
        div { class: "runs-band",
            div { class: "runs-band-head",
                span { class: "section-label",
                    if filter_empty { "此配方的批次" } else { "此输入的批次" }
                }
                span { class: "runs-count", "{runs.len()}" }
                if !filter_empty {
                    button {
                        class: "ghost-btn small",
                        onclick: move |_| state.selected_input.set(String::new()),
                        "显示配方全部批次"
                    }
                }
            }
            if runs.is_empty() {
                div { class: "empty-state",
                    div { class: "empty-glyph", crate::ui::icons::IconSparkles { size: 28 } }
                    h3 { "还没有批次" }
                    p { "在输入准备区填好内容点「生成」；每个批次都会保留当时的完整快照。" }
                }
            } else {
                div { class: "runs-list",
                    for run in runs.iter() {
                        crate::ui::feed::RunCard { key: "{run.id}", state, run: run.clone(), now: now_ms() }
                    }
                }
            }
        }
    }
}
