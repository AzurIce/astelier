//! 模板编辑区（可折叠）：prompt 模板 + 参数 + 固定参考图 + mask。
//! 显式保存，内容变化时版本 +1；历史内容通过批次快照回溯。

use crate::api::*;
use crate::app::AppState;
use crate::model::{ParamValue, Recipe};
use crate::ui::params::ParamsRow;
use crate::ui::widgets::{Dropdown, RefsStrip};
use dioxus::prelude::*;

#[component]
pub fn TemplateBand(state: AppState, recipe: Recipe) -> Element {
    let mut show_advanced = state.show_advanced;
    let mut show_readme_preview = use_signal(|| false);
    let mut readme_draft = use_signal(String::new);
    let mut readme_loaded = use_signal(|| false);
    let mut readme_dirty = use_signal(|| false);
    // 已有内容的配方默认收起；空白新配方自动展开
    let mut editor_open = use_signal(|| recipe.prompt_template.trim().is_empty());

    // 内容性编辑的工作副本：显式保存，避免每次击键都版本 +1
    let mut draft = use_signal(|| recipe.clone());

    // 「恢复模板为此快照」→ 载入草稿并标脏，用户确认保存后才成为新版本
    let mut restore = state.restore_template;
    use_effect(move || {
        let Some(snapshot) = restore.cloned() else { return };
        restore.set(None);
        draft.set(snapshot);
        editor_open.set(true);
    });

    let saved = recipe.clone();
    let content_dirty = draft.cloned().prompt_template != saved.prompt_template
        || draft.cloned().params != saved.params
        || draft.cloned().refs != saved.refs
        || draft.cloned().mask != saved.mask
        || draft.cloned().model_id != saved.model_id;

    let profile = state.profile_of(&recipe.model_id);

    // ---- README：懒加载 + 脏跟踪 ----
    let rid_readme = recipe.id.clone();
    use_effect(move || {
        let rid = rid_readme.clone();
        spawn(async move {
            if let Ok(content) = get_readme(rid).await {
                // 只在草稿未被用户改过时写入（切换配方时首次加载）
                if readme_draft.cloned().is_empty() {
                    readme_draft.set(content);
                }
                readme_loaded.set(true);
            }
        });
    });

    let rid_readme_commit = recipe.id.clone();
    let commit_readme = move |_| {
        let rid = rid_readme_commit.clone();
        let content = readme_draft.cloned();
        spawn(async move {
            let _ = crate::api::save_readme(rid, content).await;
        });
        readme_dirty.set(false);
    };

    rsx! {
        div { class: "editor-band template-band",
            div { class: "editor-head",
                button {
                    class: "ghost-btn",
                    onclick: move |_| editor_open.toggle(),
                    if editor_open() {
                        crate::ui::icons::IconChevronDown { size: 13 }
                    } else {
                        crate::ui::icons::IconChevronRight { size: 13 }
                    }
                    "模板"
                }
                if content_dirty {
                    span { class: "dirty-chip", "未保存修改 · 保存后 v{recipe.version} → v{recipe.version + 1}" }
                } else {
                    span { class: "version-chip", "v{recipe.version}" }
                }
                div { class: "editor-head-right",
                    button {
                        class: "ghost-btn small",
                        onclick: move |_| show_readme_preview.toggle(),
                        "README"
                    }
                }
            }

            if editor_open() {
                div { class: "editor-body",
                    // ---- README ----
                    div { class: "readme-section",
                        div { class: "readme-head",
                            span { class: "field-label", "README.md" }
                            span { class: "hint", "· 对应 data/recipes/{{id}}/README.md" }
                            div { class: "readme-actions",
                                if show_readme_preview() {
                                    button {
                                        class: "ghost-btn small",
                                        onclick: move |_| show_readme_preview.set(false),
                                        "编辑"
                                    }
                                } else {
                                    button {
                                        class: "ghost-btn small",
                                        onclick: move |_| show_readme_preview.set(true),
                                        "预览"
                                    }
                                    if readme_dirty() {
                                        button {
                                            class: "btn primary small",
                                            onclick: commit_readme,
                                            "保存 README"
                                        }
                                    }
                                }
                            }
                        }
                        if show_readme_preview() {
                            ReadmePreview { content: readme_draft.cloned() }
                        } else {
                            textarea {
                                class: "text-area readme-editor",
                                rows: "8",
                                spellcheck: "false",
                                placeholder: "写点配方说明：用途、变量含义、踩坑记录…（Markdown）",
                                value: "{readme_draft()}",
                                oninput: move |e| {
                                    readme_draft.set(e.value());
                                    readme_dirty.set(true);
                                },
                            }
                        }
                    }


                    div { class: "editor-grid",
                        // ---- 左：模板与参数 ----
                        div { class: "editor-col",
                            label { class: "field",
                                span { class: "field-label",
                                    "Prompt 模板 "
                                    span { class: "hint", "· {{文字槽}} {{img:图片槽}}，{{{{ = 字面 {{" }
                                }
                                textarea {
                                    class: "prompt-template",
                                    value: "{draft.cloned().prompt_template}",
                                    placeholder: "把 {{img:主体}} 里的内容画成 {{style}} 风格…",
                                    rows: "4",
                                    spellcheck: "false",
                                    oninput: move |e| {
                                        let v = e.value();
                                        draft.with_mut(|d| d.prompt_template = v);
                                    },
                                }
                            }
                            div { class: "editor-params",
                                span { class: "section-label", "参数" }
                                ParamsRow {
                                    profile: profile.clone().unwrap_or_else(|| crate::profiles::merged(&recipe.model_id, None)),
                                    params: draft.cloned().params,
                                    defs: {
                                        let p = profile.clone().unwrap_or_else(|| crate::profiles::merged(&recipe.model_id, None));
                                        let mode = crate::model::Mode::Gen;
                                        p.params_for(mode, false).into_iter().cloned().collect()
                                    },
                                    hide_groups: vec![],
                                    onset: move |(key, val): (String, ParamValue)| {
                                        draft.with_mut(|d| {
                                            d.params.insert(key, val);
                                        });
                                    },
                                }
                                button {
                                    class: "ghost-btn small advanced-toggle",
                                    onclick: move |_| show_advanced.toggle(),
                                    if show_advanced() { "收起更多参数" } else { "更多参数" }
                                    crate::ui::icons::IconChevronDown { size: 12 }
                                }
                                if show_advanced() {
                                    ParamsRow {
                                        profile: profile.clone().unwrap_or_else(|| crate::profiles::merged(&recipe.model_id, None)),
                                        params: draft.cloned().params,
                                        defs: {
                                            let p = profile.clone().unwrap_or_else(|| crate::profiles::merged(&recipe.model_id, None));
                                            p.params_for(crate::model::Mode::Gen, true).into_iter().cloned().collect()
                                        },
                                        hide_groups: vec![],
                                        onset: move |(key, val): (String, ParamValue)| {
                                            draft.with_mut(|d| {
                                                d.params.insert(key, val);
                                            });
                                        },
                                    }
                                }
                            }
                        }
                        // ---- 右：固定参考图 / mask / 模型 ----
                        div { class: "editor-col editor-col-side",
                            label { class: "field",
                                span { class: "field-label", "模型" }
                                ModelSelect { state, draft }
                            }
                            label { class: "field",
                                span { class: "field-label", "固定参考图（跟配方走）" }
                                RefsStrip {
                                    state,
                                    refs: draft.cloned().refs,
                                    on_added: move |asset: crate::model::AssetRef| {
                                        draft.with_mut(|d| d.refs.push(asset));
                                    },
                                    on_removed: move |id: String| {
                                        draft.with_mut(|d| d.refs.retain(|x| x.id != id));
                                    },
                                }
                            }
                            label { class: "field",
                                span { class: "field-label", "Mask（可选）" }
                                MaskUpload { state, draft }
                            }
                        }
                    }

                    div { class: "editor-foot",
                        if content_dirty {
                            button {
                                class: "btn primary",
                                onclick: move |_| {
                                    let d = draft.cloned();
                                    let state = state;
                                    spawn(async move {
                                        match update_recipe(d).await {
                                            Ok(saved) => {
                                                state.patch_replace_recipe(saved);
                                                state.toast("配方已保存", "ok");
                                            }
                                            Err(e) => state.toast(format!("保存失败：{e}"), "error"),
                                        }
                                    });
                                },
                                "保存配方"
                            }
                        } else {
                            span { class: "hint", "已是最新 · v{recipe.version}" }
                        }
                        button {
                            class: "ghost-btn danger-text",
                            onclick: move |_| {
                                let mut state = state;
                                let recipe_del = recipe.clone();
                                spawn(async move {
                                    let _ = delete_recipe(recipe_del.id.clone()).await;
                                    state.recipes.with_mut(|v| v.retain(|r| r.id != recipe_del.id));
                                    state.selected_recipe.set(String::new());
                                    state.toast("配方已删除（其输入一并删除）", "ok");
                                });
                            },
                            crate::ui::icons::IconTrash { size: 13 }
                            "删除配方"
                        }
                    }
                }
            }
        }
    }
}

/// Markdown 预览（服务端渲染）
#[component]
fn ReadmePreview(content: String) -> Element {
    let mut html: Signal<String> = use_signal(String::new);
    use_effect(move || {
        let md = content.clone();
        spawn(async move {
            if let Ok(rendered) = render_markdown(md).await {
                html.set(rendered);
            }
        });
    });
    rsx! {
        div {
            class: "readme-preview markdown-body",
            dangerous_inner_html: "{html()}",
        }
    }
}

// ================= 模型选择 / Mask =================

#[component]
fn ModelSelect(state: AppState, mut draft: Signal<Recipe>) -> Element {
    let items: Vec<crate::ui::widgets::DropdownItem> = state
        .profiles()
        .iter()
        .map(|p| crate::ui::widgets::DropdownItem {
            value: p.id.clone(),
            label: p.id.clone(),
            caption: Some(p.label.lines().next().unwrap_or("").to_string()),
            badges: crate::profiles::badges(p),
        })
        .collect();
    let model_id = draft.cloned().model_id;
    let label = if model_id.is_empty() { "选择模型".into() } else { model_id.clone() };
    rsx! {
        Dropdown {
            class: "model-select",
            label,
            items,
            value: model_id,
            disabled: false,
            drop_up: false,
            onpick: move |m: String| {
                draft.with_mut(|r| r.model_id = m);
            },
        }
    }
}

#[component]
fn MaskUpload(state: AppState, mut draft: Signal<Recipe>) -> Element {
    let mask = draft.cloned().mask;
    rsx! {
        div { class: "mask-row",
            if let Some(m) = mask {
                div { class: "ref-mask-chip",
                    crate::ui::icons::IconMask { size: 13 }
                    span { "mask" }
                    img { class: "mask-preview", src: "{m.url()}" }
                    button {
                        class: "icon-btn",
                        title: "移除 mask",
                        onclick: move |_| {
                            draft.with_mut(|r| r.mask = None);
                        },
                        crate::ui::icons::IconX { size: 11 }
                    }
                }
            } else {
                label { class: "ref-add mask",
                    title: "上传 mask PNG（透明区域 = 重绘区域）",
                    crate::ui::icons::IconMask { size: 15 }
                    span { "Mask" }
                    input {
                        r#type: "file",
                        accept: "image/png",
                        style: "display:none",
                        onchange: move |e| {
                            let files = e.files();
                            spawn(async move {
                                if let Some(file) = files.into_iter().next() {
                                    let name = file.name();
                                    match file.read_bytes().await {
                                        Ok(bytes) => match upload_asset(bytes.to_vec(), name).await {
                                            Ok(asset) => draft.with_mut(|r| r.mask = Some(asset)),
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
