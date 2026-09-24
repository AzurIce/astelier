//! 两种内置节点视图：生图（GenNodeView）与显示（DisplayNodeView）。
//!
//! 交互约定：节点头是拖拽把手；表单区拦截 pointerdown/keydown，
//! 避免输入触发画布拖动或 Delete 键误删节点。

use crate::app::AppState;
use crate::model::*;
use dioxus::prelude::*;
use dioxus_flow::prelude::*;

/// 生图节点：模型选择 + Prompt + 参数（折叠）+ 运行按钮 + 最近批次状态。
/// `params_open` / `on_params_open` 由 GraphPage 持有 —— dioxus-flow 会把
/// 跨越格子边界的节点卸载重挂（性能分块），组件内局部状态会丢，故外提。
/// `entering`：刚创建的节点播一次入场动画（限时 class，跨格重挂不重放）。
#[component]
pub fn GenNodeView(
    state: AppState,
    ctx: NodeViewCtx<NodeData>,
    graph_id: String,
    params_open: bool,
    on_params_open: EventHandler<bool>,
    entering: bool,
    on_update: EventHandler<GenNodeData>,
    on_run: EventHandler<()>,
) -> Element {
    let NodeData::Gen(gen) = &ctx.node.data else {
        return rsx! {};
    };
    let gen = gen.clone();

    let provider = state.config().and_then(|cfg| {
        cfg.providers
            .iter()
            .find(|p| p.id == gen.provider_id)
            .cloned()
    });
    let model_label = if gen.model_id.is_empty() {
        "选择模型".to_string()
    } else {
        gen.model_id.clone()
    };
    let model_items: Vec<crate::ui::widgets::DropdownItem> = provider
        .as_ref()
        .map(|p| {
            p.models
                .iter()
                .map(|m| crate::ui::widgets::DropdownItem {
                    value: m.clone(),
                    label: m.clone(),
                    caption: None,
                    badges: vec![],
                })
                .collect()
        })
        .unwrap_or_default();
    let has_models = !model_items.is_empty();

    let profile = state.profile_of(&gen.model_id);
    let param_defs: Vec<ParamDef> = profile
        .as_ref()
        .map(|p| {
            p.params_for(Mode::Gen, false)
                .into_iter()
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let has_params = !param_defs.is_empty();

    // 最近一次该节点的批次（runs 按时间倒序，取首个命中）
    let nid = ctx.node.id.clone();
    let last_run: Option<Run> = state
        .runs()
        .into_iter()
        .find(|r| {
            r.graph_id.as_deref() == Some(graph_id.as_str())
                && r.node_id.as_deref() == Some(nid.as_str())
        });
    let is_running = last_run
        .as_ref()
        .is_some_and(|r| r.status == RunStatus::Running);

    // 供各回调使用的独立克隆
    let gen_dd = gen.clone();
    let gen_prompt = gen.clone();
    let gen_params = gen.clone();

    rsx! {
        div { class: if entering { "gnv gnv-gen gnv-enter" } else { "gnv gnv-gen" },
            Handle { kind: HandleKind::Target, position: Side::Left }
            Handle { kind: HandleKind::Source, position: Side::Right }
            div { class: "gnv-head",
                span { class: "gnv-kind gnv-kind-gen",
                    crate::ui::icons::IconSparkles { size: 12 }
                    "生图"
                }
                button {
                    class: if is_running { "gnv-run running" } else { "gnv-run" },
                    title: "用当前内容发起生成",
                    onpointerdown: move |e| e.stop_propagation(),
                    onclick: move |e| {
                        e.stop_propagation();
                        on_run(());
                    },
                    if is_running {
                        span { class: "gnv-run-spin", crate::ui::icons::IconLoader { size: 12 } }
                    } else {
                        crate::ui::icons::IconPlay { size: 12 }
                    }
                    "生成"
                }
            }
            div { class: "gnv-form",
                onpointerdown: move |e| e.stop_propagation(),
                div { class: "gnv-model",
                    crate::ui::widgets::Dropdown {
                        class: "gnv-model-dd",
                        label: model_label,
                        caption: None,
                        items: model_items,
                        value: gen.model_id.clone(),
                        disabled: !has_models,
                        drop_up: false,
                        onpick: move |m: String| {
                            let mut g = gen_dd.clone();
                            g.model_id = m;
                            on_update(g);
                        },
                    }
                }
                textarea {
                    class: "gnv-prompt",
                    value: "{gen.prompt}",
                    placeholder: "描述要生成的画面…",
                    rows: 3,
                    spellcheck: "false",
                    oninput: move |e| {
                        let mut g = gen_prompt.clone();
                        g.prompt = e.value();
                        on_update(g);
                    },
                    onkeydown: move |e| e.stop_propagation(),
                }
                if let Some(p) = &profile {
                    if has_params {
                        div { class: if params_open { "gnv-params open" } else { "gnv-params" },
                            button {
                                class: "gnv-params-head",
                                title: "展开 / 收起参数",
                                onpointerdown: move |e| e.stop_propagation(),
                                onclick: move |e| {
                                    e.stop_propagation();
                                    on_params_open(!params_open);
                                },
                                span { class: if params_open { "gnv-params-caret open" } else { "gnv-params-caret" },
                                    crate::ui::icons::IconChevronRight { size: 11 }
                                }
                                "参数"
                            }
                            if params_open {
                                div { class: "gnv-params-body",
                                    crate::ui::params::ParamsRow {
                                        profile: p.clone(),
                                        params: gen.params.clone(),
                                        defs: param_defs,
                                        hide_groups: vec!["safety".into()],
                                        onset: move |(k, v): (String, ParamValue)| {
                                            let mut g = gen_params.clone();
                                            g.params.insert(k, v);
                                            on_update(g);
                                        },
                                    }
                                }
                            }
                        }
                    }
                } else if !gen.model_id.is_empty() {
                    div { class: "gnv-note", "切换到该模型所在 Provider 后可编辑参数" }
                }
            }
            {match &last_run {
                Some(run) => rsx! {
                    div { class: match run.status {
                        RunStatus::Running => "gnv-status running",
                        RunStatus::Done => "gnv-status ok",
                        RunStatus::Error => "gnv-status err",
                    },
                        {match run.status {
                            RunStatus::Running => rsx! { "生成中…" },
                            RunStatus::Done => rsx! {
                                {match run.duration_ms {
                                    Some(ms) => format!("✓ {:.1}s · {} 张", ms as f64 / 1000.0, run.images.len()),
                                    None => format!("✓ {} 张", run.images.len()),
                                }}
                            },
                            RunStatus::Error => {
                                let err_text = run.error.clone().unwrap_or_default();
                                rsx! {
                                    span { title: "{err_text}", "✗ {err_text}" }
                                }
                            },
                        }}
                    }
                },
                None => rsx! {},
            }}
        }
    }
}

/// 显示节点：展示直接上游生图节点最近一次批次的输出，点击放大。
#[component]
pub fn DisplayNodeView(
    state: AppState,
    ctx: NodeViewCtx<NodeData>,
    graph_id: String,
    upstream_gens: Vec<String>,
    entering: bool,
) -> Element {
    let _ = &ctx;
    let runs_all = state.runs();
    // 每个上游生图节点取最近一次批次
    let latest: Vec<Run> = upstream_gens
        .iter()
        .filter_map(|gid| {
            runs_all
                .iter()
                .find(|r| {
                    r.graph_id.as_deref() == Some(graph_id.as_str())
                        && r.node_id.as_deref() == Some(gid.as_str())
                })
                .cloned()
        })
        .collect();

    let any_running = latest.iter().any(|r| r.status == RunStatus::Running);
    let has_images = latest.iter().any(|r| !r.images.is_empty());

    rsx! {
        div { class: if entering { "gnv gnv-display gnv-enter" } else { "gnv gnv-display" },
            Handle { kind: HandleKind::Target, position: Side::Left }
            div { class: "gnv-head",
                span { class: "gnv-kind",
                    crate::ui::icons::IconImage { size: 12 }
                    "显示"
                }
                if any_running {
                    span { class: "gnv-disp-running",
                        crate::ui::icons::IconLoader { size: 11 }
                    }
                }
            }
            if upstream_gens.is_empty() {
                div { class: "gnv-empty", "从生图节点拉一条连线过来" }
            } else if !has_images && !any_running {
                div { class: "gnv-empty", "等待生图节点运行" }
            } else {
                div { class: "dnv-grid",
                    for run in latest.iter().cloned() {
                        for (i, img) in run.images.iter().enumerate() {
                            div { key: "{img.id}", class: "dnv-tile",
                                DnvImage { asset: img.clone(), run_id: run.id.clone(), index: i, state }
                            }
                        }
                        if run.status == RunStatus::Running {
                            for i in run.images.len()..expected_n(&run) {
                                div { key: "ph-{i}", class: "dnv-tile placeholder",
                                    div { class: "shimmer" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn expected_n(run: &Run) -> usize {
    run.effective_params()
        .get("n")
        .and_then(|v| match v {
            ParamValue::Number(n) => Some(*n as usize),
            _ => None,
        })
        .unwrap_or(1)
        .clamp(1, 10)
}

/// 单张输出图（独立组件：click 处理器要求 'static，借不来）
#[component]
fn DnvImage(state: AppState, asset: AssetRef, run_id: String, index: usize) -> Element {
    rsx! {
        img {
            src: "{asset.url()}",
            loading: "lazy",
            onclick: move |e| {
                e.stop_propagation();
                state.lightbox.set(Some((run_id.clone(), index)));
            },
        }
    }
}
