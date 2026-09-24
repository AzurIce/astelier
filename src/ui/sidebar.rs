//! 左侧：文件夹 > 节点图 两级树。
//!
//! - 点击图 → 打开节点图画布
//! - 图可拖入文件夹；双击/铅笔重命名（标题与图页工具栏同步）

use crate::api::*;
use crate::app::AppState;
use crate::model::fmt::relative_time;
use crate::model::{Graph, GraphGroup};
use crate::util::now_ms;
use dioxus::prelude::*;

const LOOSE_ZONE: &str = "__loose__";

#[component]
pub fn Sidebar(state: AppState) -> Element {
    let mut selected_graph = state.selected_graph;
    let mut graphs = state.graphs;
    let mut groups = state.groups;
    let mut drag_id = use_signal(|| None::<String>);
    let mut drop_hover = use_signal(|| None::<String>);

    let items = state.graphs();
    let groups_list = state.groups();

    let loose: Vec<Graph> = items
        .iter()
        .filter(|g| g.group_id.is_none())
        .cloned()
        .collect();

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-head",
                span { class: "section-label", "节点图" }
                span { class: "sidebar-count", "{items.len()}" }
                button {
                    class: "icon-btn",
                    title: "新建分组",
                    onclick: move |_| {
                        spawn(async move {
                            match create_group("新建分组".into()).await {
                                Ok(group) => {
                                    groups.with_mut(|v| {
                                        v.push(group.clone());
                                        v.sort_by_key(|g| g.created_at);
                                    });
                                    state.toast("已创建分组，双击名称可重命名", "ok");
                                }
                                Err(e) => state.toast(format!("新建分组失败：{e}"), "error"),
                            }
                        });
                    },
                    crate::ui::icons::IconFolderPlus { size: 15 }
                }
                button {
                    class: "icon-btn",
                    title: "新建节点图",
                    onclick: move |_| {
                        spawn(async move {
                            match create_graph().await {
                                Ok(graph) => {
                                    graphs.write().insert(0, graph.clone());
                                    state.select_graph(&graph.id);
                                }
                                Err(e) => state.toast(format!("新建失败：{e}"), "error"),
                            }
                        });
                    },
                    crate::ui::icons::IconPlus { size: 15 }
                }
            }

            if items.is_empty() && groups_list.is_empty() {
                div { class: "sidebar-empty",
                    "还没有节点图。"
                    br {}
                    "点右上 + 新建一个图开始创作。"
                }
            } else {
                div { class: "input-list",
                    // ---- 未分组图 ----
                    for graph in loose.iter() {
                        GraphRow {
                            key: "{graph.id}",
                            state,
                            graph: graph.clone(),
                            drag_id,
                        }
                    }
                    // ---- 未分组放置区 ----
                    if !groups_list.is_empty() {
                        div {
                            class: if drop_hover.cloned() == Some(LOOSE_ZONE.into()) {
                                "group-dropzone hover"
                            } else {
                                "group-dropzone"
                            },
                            ondragover: move |e| {
                                if drag_id.cloned().is_some() {
                                    e.prevent_default();
                                    drop_hover.set(Some(LOOSE_ZONE.into()));
                                }
                            },
                            ondragleave: move |_| {
                                if drop_hover.cloned() == Some(LOOSE_ZONE.into()) {
                                    drop_hover.set(None);
                                }
                            },
                            ondrop: move |e| {
                                e.prevent_default();
                                if let Some(id) = drag_id.cloned() {
                                    let id_spawn = id.clone();
                                    // 先更新本地（侧边栏按本地状态渲染，只写
                                    // 服务端的话 UI 永远不动），再落盘
                                    graphs.with_mut(|v| {
                                        if let Some(g) = v.iter_mut().find(|g| g.id == id) {
                                            g.group_id = None;
                                        }
                                    });
                                    spawn(async move {
                                        let _ = set_graph_group(id_spawn, None).await;
                                    });
                                    selected_graph.set(id);
                                }
                                drop_hover.set(None);
                                drag_id.set(None);
                            },
                            "未分组 · 拖入"
                        }
                    }
                    // ---- 分组 ----
                    for g in groups_list.iter() {
                        GroupBlock {
                            key: "{g.id}",
                            state,
                            group: g.clone(),
                            drag_id,
                            drop_hover,
                        }
                    }
                }
            }
        }
    }
}

fn begin_graph_rename(mut draft: Signal<String>, mut renaming: Signal<bool>, name: String) {
    draft.set(name);
    renaming.set(true);
}

fn commit_graph_rename(
    state: AppState,
    mut renaming: Signal<bool>,
    draft: Signal<String>,
    gid: String,
) {
    if !renaming.cloned() {
        return;
    }
    renaming.set(false);
    let name = draft.cloned().trim().to_string();
    let name = if name.is_empty() { "未命名图".to_string() } else { name };
    if let Some(graph) = state.graphs().into_iter().find(|g| g.id == gid) {
        state.patch_graph(graph, move |g| g.title = name);
    }
}

/// 图行（树叶子）
#[component]
fn GraphRow(
    state: AppState,
    graph: Graph,
    drag_id: Signal<Option<String>>,
) -> Element {
    let mut renaming = use_signal(|| false);
    let mut draft = use_signal(String::new);

    let gid_sel = graph.id.clone();
    let gid_drag = graph.id.clone();
    let gid_enter = graph.id.clone();
    let gid_blur = graph.id.clone();
    let gid_del = graph.id.clone();
    let name_dbl = graph.title.clone();
    let name_pencil = graph.title.clone();
    let is_sel = state.selected_graph_id() == graph.id;
    let time = relative_time(graph.updated_at, now_ms());

    rsx! {
        div {
            class: if is_sel { "recipe-head selected" } else { "recipe-head" },
            draggable: true,
            onclick: move |_| state.select_graph(&gid_sel),
            ondragstart: move |_| drag_id.set(Some(gid_drag.clone())),
            ondragend: move |_| drag_id.set(None),
            if renaming() {
                input {
                    class: "input-rename",
                    value: "{draft}",
                    autofocus: true,
                    spellcheck: "false",
                    onclick: move |e| e.stop_propagation(),
                    oninput: move |e| draft.set(e.value()),
                    onkeydown: move |e| {
                        use keyboard_types::Key;
                        match e.key() {
                            Key::Enter => {
                                e.prevent_default();
                                commit_graph_rename(state, renaming, draft.clone(), gid_enter.clone());
                            }
                            Key::Escape => renaming.set(false),
                            _ => {}
                        }
                    },
                    onblur: move |_| {
                        commit_graph_rename(state, renaming, draft.clone(), gid_blur.clone());
                    },
                }
            } else {
                span { class: "recipe-chevron",
                    crate::ui::icons::IconWorkflow { size: 12 }
                }
                span {
                    class: "recipe-name",
                    title: "双击重命名",
                    ondoubleclick: move |e| {
                        e.stop_propagation();
                        begin_graph_rename(draft, renaming, name_dbl.clone());
                    },
                    "{graph.display_title()}"
                }
            }
            if !renaming() {
                div { class: "recipe-actions",
                    button {
                        class: "icon-btn input-item-action",
                        title: "重命名",
                        onclick: move |e| {
                            e.stop_propagation();
                            begin_graph_rename(draft, renaming, name_pencil.clone());
                        },
                        crate::ui::icons::IconPencil { size: 12 }
                    }
                    button {
                        class: "icon-btn input-item-action",
                        title: "删除节点图",
                        onclick: move |e| {
                            e.stop_propagation();
                            let mut state = state;
                            let gid = gid_del.clone();
                            spawn(async move {
                                let _ = delete_graph(gid.clone()).await;
                                state.graphs.with_mut(|v| v.retain(|g| g.id != gid));
                                if state.selected_graph_id() == gid {
                                    state.selected_graph.set(String::new());
                                }
                                state.toast("节点图已删除", "ok");
                            });
                        },
                        crate::ui::icons::IconTrash { size: 12 }
                    }
                }
                span { class: "recipe-meta", "{time}" }
            }
        }
    }
}

/// 文件夹：图的拖拽放置目标
#[component]
fn GroupBlock(
    state: AppState,
    group: GraphGroup,
    drag_id: Signal<Option<String>>,
    drop_hover: Signal<Option<String>>,
) -> Element {
    let mut open = use_signal(|| true);

    let gid = group.id.clone();
    let gid_hover = gid.clone();
    let gid_leave = gid.clone();
    let gid_drop = gid.clone();
    let is_hover = drop_hover.cloned() == Some(gid.clone());
    let graphs: Vec<Graph> = state
        .graphs()
        .into_iter()
        .filter(|g| g.group_id.as_deref() == Some(group.id.as_str()))
        .collect();

    rsx! {
        div { class: "group-block",
            div {
                class: if is_hover { "group-head drop-hover" } else { "group-head" },
                onclick: move |_| open.toggle(),
                ondragover: move |e| {
                    if drag_id.cloned().is_some() {
                        e.prevent_default();
                        drop_hover.set(Some(gid_hover.clone()));
                    }
                },
                ondragleave: move |_| {
                    if drop_hover.cloned() == Some(gid_leave.clone()) {
                        drop_hover.set(None);
                    }
                },
                ondrop: move |e| {
                    e.prevent_default();
                    if let Some(id) = drag_id.cloned() {
                        let gid_drop2 = gid_drop.clone();
                        // 先更新本地（侧边栏按本地状态渲染，只写服务端的话
                        // UI 永远不动），再落盘
                        let mut state = state;
                        let id2 = id.clone();
                        state.graphs.with_mut(|v| {
                            if let Some(g) = v.iter_mut().find(|g| g.id == id2) {
                                g.group_id = Some(gid_drop2.clone());
                            }
                        });
                        spawn(async move {
                            let _ = set_graph_group(id, Some(gid_drop2)).await;
                        });
                    }
                    drop_hover.set(None);
                    drag_id.set(None);
                },
                span { class: if open() { "group-chevron open" } else { "group-chevron" },
                    if open() {
                        crate::ui::icons::IconChevronDown { size: 13 }
                    } else {
                        crate::ui::icons::IconChevronRight { size: 13 }
                    }
                }
                span { class: "group-icon", crate::ui::icons::IconFolder { size: 13 } }
                span { class: "group-name", "{group.name}" }
                span { class: "group-count", "{graphs.len()}" }
                button {
                    class: "icon-btn input-item-action group-delete",
                    title: "删除分组（图回到未分组）",
                    onclick: move |e| {
                        e.stop_propagation();
                        let mut state = state;
                        let gid_del = gid.clone();
                        spawn(async move {
                            let _ = delete_group(gid_del.clone()).await;
                            state.groups.with_mut(|v| v.retain(|g| g.id != gid_del));
                            state.graphs.with_mut(|v| {
                                for g in v.iter_mut() {
                                    if g.group_id.as_deref() == Some(gid_del.as_str()) {
                                        g.group_id = None;
                                    }
                                }
                            });
                        });
                    },
                    crate::ui::icons::IconTrash { size: 12 }
                }
            }
            if open() {
                div { class: "group-items",
                    if graphs.is_empty() {
                        div { class: "group-empty", "空分组 · 拖入节点图" }
                    }
                    for graph in graphs.iter() {
                        GraphRow {
                            key: "{graph.id}",
                            state,
                            graph: graph.clone(),
                            drag_id,
                        }
                    }
                }
            }
        }
    }
}
