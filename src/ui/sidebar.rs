//! 左侧：文件夹 > 配方 > 输入 三级树。
//!
//! - 点击配方 → 打开配方页（编辑 + 全部批次）
//! - 点击输入 → 配方页 + 载入该输入（批次过滤到该输入）
//! - 配方可拖入文件夹；双击/铅笔重命名（标题与配方页编辑区同步）

use crate::api::*;
use crate::app::AppState;
use crate::model::fmt::relative_time;
use crate::model::{InputGroup, Recipe, RecipeInput};
use crate::util::now_ms;
use dioxus::prelude::*;

const LOOSE_ZONE: &str = "__loose__";

#[component]
pub fn Sidebar(state: AppState) -> Element {
    let mut selected_recipe = state.selected_recipe;
    let mut recipes = state.recipes;
    let mut groups = state.groups;
    let mut drag_id = use_signal(|| None::<String>);
    let mut drop_hover = use_signal(|| None::<String>);

    let items = state.recipes();
    let groups_list = state.groups();

    let loose: Vec<Recipe> = items
        .iter()
        .filter(|r| r.group_id.is_none())
        .cloned()
        .collect();

    rsx! {
        aside { class: "sidebar",
            div { class: "sidebar-head",
                span { class: "section-label", "配方" }
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
                    title: "新建配方",
                    onclick: move |_| {
                        spawn(async move {
                            let (pid, mid) = state
                                .config()
                                .and_then(|cfg| cfg.active().map(|p| {
                                    (p.id.clone(), p.models.first().cloned().unwrap_or_default())
                                }))
                                .unwrap_or_default();
                            match create_recipe(pid, mid).await {
                                Ok(recipe) => {
                                    recipes.write().insert(0, recipe.clone());
                                    state.select_recipe(&recipe.id);
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
                    "还没有配方。"
                    br {}
                    "点右上 + 新建一个配方开始创作。"
                }
            } else {
                div { class: "input-list",
                    // ---- 未分组配方 ----
                    for recipe in loose.iter() {
                        RecipeRow {
                            key: "{recipe.id}",
                            state,
                            recipe: recipe.clone(),
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
                                    spawn(async move {
                                        let _ = set_recipe_group(id_spawn, None).await;
                                    });
                                    selected_recipe.set(id);
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

fn begin_recipe_rename(mut draft: Signal<String>, mut renaming: Signal<bool>, name: String) {
    draft.set(name);
    renaming.set(true);
}

fn commit_recipe_rename(
    state: AppState,
    mut renaming: Signal<bool>,
    draft: Signal<String>,
    rid: String,
) {
    if !renaming.cloned() {
        return;
    }
    renaming.set(false);
    let name = draft.cloned().trim().to_string();
    let name = if name.is_empty() { "未命名".to_string() } else { name };
    state.patch_recipe(&rid, move |r| r.title = Some(name));
}

/// 配方行 + 展开的输入列表（仅选中配方的输入可见）
#[component]
fn RecipeRow(
    state: AppState,
    recipe: Recipe,
    drag_id: Signal<Option<String>>,
) -> Element {
    let mut renaming = use_signal(|| false);
    let mut draft = use_signal(String::new);
    let mut selected_input = state.selected_input;

    let rid_sel = recipe.id.clone();
    let rid_drag = recipe.id.clone();
    let rid_enter = recipe.id.clone();
    let rid_blur = recipe.id.clone();
    let rid_del = recipe.id.clone();
    let name_dbl = recipe.display_title();
    let name_pencil = recipe.display_title();
    let is_sel = state.selected_recipe_id() == recipe.id;
    let time = relative_time(recipe.updated_at, now_ms());

    rsx! {
        div { class: "recipe-block",
            div {
                class: if is_sel { "recipe-head selected" } else { "recipe-head" },
                draggable: true,
                onclick: move |_| state.select_recipe(&rid_sel),
                ondragstart: move |_| drag_id.set(Some(rid_drag.clone())),
                ondragend: move |_| drag_id.set(None),
                span { class: "recipe-chevron",
                    if is_sel {
                        crate::ui::icons::IconChevronDown { size: 12 }
                    } else {
                        crate::ui::icons::IconChevronRight { size: 12 }
                    }
                }
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
                                    commit_recipe_rename(state, renaming, draft.clone(), rid_enter.clone());
                                }
                                Key::Escape => renaming.set(false),
                                _ => {}
                            }
                        },
                        onblur: move |_| {
                            commit_recipe_rename(state, renaming, draft.clone(), rid_blur.clone());
                        },
                    }
                } else {
                    span {
                        class: "recipe-name",
                        title: "双击重命名",
                        ondoubleclick: move |e| {
                            e.stop_propagation();
                            begin_recipe_rename(draft, renaming, name_dbl.clone());
                        },
                        "{recipe.display_title()}"
                    }
                }
                div { class: "recipe-actions",
                    button {
                        class: "icon-btn input-item-action",
                        title: "重命名",
                        onclick: move |e| {
                            e.stop_propagation();
                            begin_recipe_rename(draft, renaming, name_pencil.clone());
                        },
                        crate::ui::icons::IconPencil { size: 12 }
                    }
                    button {
                        class: "icon-btn input-item-action",
                        title: "删除配方",
                        onclick: move |e| {
                            e.stop_propagation();
                            let mut state = state;
                            let rid = rid_del.clone();
                            spawn(async move {
                                let _ = delete_recipe(rid.clone()).await;
                                state.recipes.with_mut(|v| v.retain(|r| r.id != rid));
                                if state.selected_recipe_id() == rid {
                                    state.selected_recipe.set(String::new());
                                    state.selected_input.set(String::new());
                                }
                                state.toast("配方已删除", "ok");
                            });
                        },
                        crate::ui::icons::IconTrash { size: 12 }
                    }
                }
                span { class: "recipe-meta", "{time}" }
            }
            if is_sel {
                div { class: "recipe-inputs",
                    // 首项 = 未发送的草稿；生成时才会真正保存为输入
                    button {
                        class: if state.selected_input_id().is_empty() {
                            "input-leaf selected"
                        } else {
                            "input-leaf"
                        },
                        title: "填好内容点生成后，才会真正保存为输入",
                        onclick: move |_| {
                            selected_input.set(String::new());
                        },
                        crate::ui::icons::IconSparkles { size: 11 }
                        span { class: "input-leaf-name", "新建输入" }
                    }
                    for input in state.recipe_inputs().iter() {
                        InputLeaf {
                            key: "{input.id}",
                            state,
                            input: input.clone(),
                            selected: input.id == state.selected_input_id(),
                        }
                    }
                }
            }
        }
    }
}

fn begin_input_rename(mut draft: Signal<String>, mut renaming: Signal<bool>, title: Option<String>) {
    draft.set(title.unwrap_or_default());
    renaming.set(true);
}

fn commit_input_rename(
    state: AppState,
    mut renaming: Signal<bool>,
    draft: Signal<String>,
    id: String,
) {
    if !renaming.cloned() {
        return;
    }
    renaming.set(false);
    let t = draft.cloned().trim().to_string();
    state.mutate_input(&id, move |i| i.title = if t.is_empty() { None } else { Some(t) });
}

/// 输入叶子行：点击载入、重命名、删除
#[component]
fn InputLeaf(
    state: AppState,
    input: RecipeInput,
    selected: bool,
) -> Element {
    let mut renaming = use_signal(|| false);
    let mut draft = use_signal(String::new);

    let id = input.id.clone();
    let id_key = id.clone();
    let id_blur = id.clone();
    let id_click = id.clone();
    let rid_click = input.recipe_id.clone();
    let rid_del = input.recipe_id.clone();
    let title_dbl = input.title.clone();
    let title_pencil = input.title.clone();
    let id_del = input.id.clone();
    let title = input.display_title(0);

    rsx! {
        div {
            class: if selected { "input-leaf selected" } else { "input-leaf" },
            onclick: move |_| state.select_input(&rid_click, &id_click),
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
                                commit_input_rename(state, renaming, draft.clone(), id_key.clone());
                            }
                            Key::Escape => renaming.set(false),
                            _ => {}
                        }
                    },
                    onblur: move |_| {
                        commit_input_rename(state, renaming, draft.clone(), id_blur.clone());
                    },
                }
            } else {
                span {
                    class: "input-leaf-name",
                    title: "双击重命名",
                    ondoubleclick: move |e| {
                        e.stop_propagation();
                        begin_input_rename(draft, renaming, title_dbl.clone());
                    },
                    "{title}"
                }
            }
            div { class: "input-leaf-actions",
                button {
                    class: "icon-btn input-item-action",
                    title: "重命名",
                    onclick: move |e| {
                        e.stop_propagation();
                        begin_input_rename(draft, renaming, title_pencil.clone());
                    },
                    crate::ui::icons::IconPencil { size: 11 }
                }
                button {
                    class: "icon-btn input-item-action",
                    title: "删除输入",
                    onclick: move |e| {
                        e.stop_propagation();
                        let mut state = state;
                        let id_del2 = id_del.clone();
                        let rid_del2 = rid_del.clone();
                        spawn(async move {
                            let _ = delete_input(rid_del2, id_del2.clone()).await;
                            state.recipe_inputs.write().retain(|x| x.id != id_del2);
                            if state.selected_input_id() == id_del2 {
                                state.selected_input.set(String::new());
                            }
                        });
                    },
                    crate::ui::icons::IconTrash { size: 11 }
                }
            }
        }
    }
}

/// 文件夹：配方的拖拽放置目标
#[component]
fn GroupBlock(
    state: AppState,
    group: InputGroup,
    drag_id: Signal<Option<String>>,
    drop_hover: Signal<Option<String>>,
) -> Element {
    let mut open = use_signal(|| true);

    let gid = group.id.clone();
    let gid_hover = gid.clone();
    let gid_leave = gid.clone();
    let gid_drop = gid.clone();
    let is_hover = drop_hover.cloned() == Some(gid.clone());
    let recipes: Vec<Recipe> = state
        .recipes()
        .into_iter()
        .filter(|r| r.group_id.as_deref() == Some(group.id.as_str()))
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
                        spawn(async move {
                            let _ = set_recipe_group(id, Some(gid_drop2)).await;
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
                span { class: "group-count", "{recipes.len()}" }
                button {
                    class: "icon-btn input-item-action group-delete",
                    title: "删除分组（配方回到未分组）",
                    onclick: move |e| {
                        e.stop_propagation();
                        let mut state = state;
                        let gid_del = gid.clone();
                        spawn(async move {
                            let _ = delete_group(gid_del.clone()).await;
                            state.groups.with_mut(|v| v.retain(|g| g.id != gid_del));
                            state.recipes.with_mut(|v| {
                                for r in v.iter_mut() {
                                    if r.group_id.as_deref() == Some(gid_del.as_str()) {
                                        r.group_id = None;
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
                    if recipes.is_empty() {
                        div { class: "group-empty", "空分组 · 拖入配方" }
                    }
                    for recipe in recipes.iter() {
                        RecipeRow {
                            key: "{recipe.id}",
                            state,
                            recipe: recipe.clone(),
                            drag_id,
                        }
                    }
                }
            }
        }
    }
}
