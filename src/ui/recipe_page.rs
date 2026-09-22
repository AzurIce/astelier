//! 配方页：顶部编辑区（可折叠）+ 此配方的 Run 列表 + 底部快捷运行条。
//!
//! 选中层级：配方 → 编辑配方；输入 → 快捷框载入该输入，中栏过滤其 Run。

use crate::api::*;
use crate::app::AppState;
use crate::model::{AssetRef, ParamValue, Recipe, RecipeInput, Run};
use crate::ui::params::ParamsRow;
use crate::ui::widgets::Dropdown;
use crate::util::now_ms;
use dioxus::prelude::*;
use std::collections::BTreeMap;

#[component]
pub fn RecipePage(state: AppState, recipe: Recipe) -> Element {
    let editor_open = use_signal(|| true);
    let runs_all = state.runs();

    // 中栏过滤：选中输入 → 只看它的 Run；只选配方 → 看配方全部 Run
    let input_filter = state.selected_input_id();
    let runs: Vec<Run> = runs_all
        .iter()
        .filter(|r| {
            r.recipe_id == recipe.id
                && (input_filter.is_empty() || r.input_id.as_deref() == Some(input_filter.as_str()))
        })
        .cloned()
        .collect();

    // 选中的输入（快捷框载入其值）
    let selected_input = state.selected_input();

    rsx! {
        div { class: "stage recipe-page",
            EditorBand { state, recipe: recipe.clone(), editor_open }

            div { class: "runs-band",
                div { class: "runs-band-head",
                    span { class: "section-label",
                        if input_filter.is_empty() { "此配方的批次" } else { "此输入的批次" }
                    }
                    span { class: "runs-count", "{runs.len()}" }
                    if !input_filter.is_empty() {
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
                        p { "在下方快捷运行条里填好内容，或先在上方编辑配方。" }
                    }
                } else {
                    div { class: "runs-list",
                        for run in runs.iter() {
                            crate::ui::feed::RunCard { key: "{run.id}", state, run: run.clone(), now: now_ms() }
                        }
                    }
                }
            }

            RunBar { state, recipe, selected_input }
        }
    }
}

// ================= 编辑区 =================

#[component]
fn EditorBand(state: AppState, recipe: Recipe, editor_open: Signal<bool>) -> Element {
    let mut show_advanced = state.show_advanced;
    let mut show_readme_preview = use_signal(|| false);
    let mut readme_draft = use_signal(String::new);
    let mut readme_loaded = use_signal(|| false);
    let mut readme_dirty = use_signal(|| false);

    // 内容性编辑的工作副本：显式保存，避免每次击键都版本 +1
    let mut draft = use_signal(|| recipe.clone());

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
        div { class: "editor-band",
            div { class: "editor-head",
                button {
                    class: "ghost-btn",
                    onclick: move |_| editor_open.toggle(),
                    if editor_open() {
                        crate::ui::icons::IconChevronDown { size: 13 }
                    } else {
                        crate::ui::icons::IconChevronRight { size: 13 }
                    }
                    "编辑配方"
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
                                RefsStrip { state, draft }
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

// ================= 快捷运行条 =================

#[component]
fn RunBar(state: AppState, recipe: Recipe, selected_input: Option<RecipeInput>) -> Element {
    let (mut variables, mut slot_images, mut input_id) = run_bar_state();

    // 选中输入变化 → 载入其值
    use_effect(move || {
        let iid = state.selected_input_id();
        if let Some(input) = state.selected_input() {
            input_id.set(iid);
            variables.set(input.variables.clone());
            slot_images.set(input.images.clone());
        } else if iid.is_empty() {
            input_id.set(String::new());
            variables.set(Default::default());
            slot_images.set(Default::default());
        }
    });

    let (vars, img_slots) = crate::model::template_variables(&recipe.prompt_template);

    let recipe_gen = recipe.clone();
    let generate = move |_| {
        let rendered = crate::model::render_recipe(
            &recipe_gen.prompt_template,
            &variables.cloned(),
            &slot_images.cloned(),
            recipe_gen.refs.len(),
        );
        if !rendered.is_complete() {
            let mut problems = vec![];
            if !rendered.missing_vars.is_empty() {
                problems.push(format!("未填变量：{}", rendered.missing_vars.join("、")));
            }
            if !rendered.missing_imgs.is_empty() {
                problems.push(format!("未绑定图片槽：{}", rendered.missing_imgs.join("、")));
            }
            state.toast(problems.join("；"), "error");
            return;
        }
        if let Some(profile) = state.profile_of(&recipe_gen.model_id) {
            let draft_for_validate = recipe_gen.clone();
            if let Err(msg) = draft_for_validate.validate_params(&profile) {
                state.toast(msg, "error");
                return;
            }
        }
        let values = variables.cloned();
        let images = slot_images.cloned();
        let rid = recipe_gen.id.clone();
        let iid = input_id.cloned();
        let mut state = state;
        spawn(async move {
            let input = RecipeInput {
                id: iid.clone(),
                recipe_id: rid.clone(),
                title: None,
                variables: values,
                images,
                created_at: 0,
                updated_at: 0,
            };
            if !iid.is_empty() {
                let _ = update_input(input.clone()).await;
            }
            match start_run(rid, input).await {
                Ok(run) => {
                    if let Some(new_iid) = &run.input_id {
                        input_id.set(new_iid.clone());
                        if let Ok(list) = list_inputs(run.recipe_id.clone()).await {
                            state.recipe_inputs.set(list);
                        }
                    }
                    state.runs.with_mut(|v| v.insert(0, run));
                }
                Err(e) => state.toast(format!("无法开始生成：{e}"), "error"),
            }
        });
    };

    let var_fields: Vec<(String, String)> = vars
        .iter()
        .map(|name| {
            let val = variables
                .cloned()
                .get(name)
                .and_then(|v| v.clone().into())
                .unwrap_or_default();
            (name.clone(), val)
        })
        .collect();

    rsx! {
        div { class: "run-bar",
            div { class: "run-bar-main",
                if vars.is_empty() && img_slots.is_empty() {
                    span { class: "hint", "模板里还没有槽位 — 在配方模板中写 {{文字槽}} 或 {{img:图片槽}} 后，这里会出现对应的输入框。" }
                }
                for (name, val_now) in var_fields.iter().cloned() {
                    div { class: "slot-field",
                        span { class: "param-label slot-name", "{{{name}}}" }
                        input {
                            class: "text-input slot-input",
                            r#type: "text",
                            value: "{val_now}",
                            placeholder: "{name} 的值",
                            oninput: move |e| {
                                let v = e.value();
                                variables.with_mut(|m| {
                                    m.insert(name.clone(), v);
                                });
                            },
                        }
                    }
                }
                for slot in img_slots.iter().cloned() {
                    {
                        let slot_bind = slot.clone();
                        let slot_clear = slot.clone();
                        rsx! {
                            SlotImageChip {
                                state,
                                slot: slot.clone(),
                                bound: slot_images.cloned().get(&slot).cloned(),
                                onbind: move |asset: AssetRef| {
                                    slot_images.with_mut(|m| {
                                        m.insert(slot_bind.clone(), asset);
                                    });
                                },
                                onclear: move |_| {
                                    slot_images.with_mut(|m| {
                                        m.remove(&slot_clear);
                                    });
                                },
                            }
                        }
                    }
                }
                div { class: "rendered-preview",
                    crate::ui::icons::IconSparkles { size: 11 }
                    {crate::model::render_recipe(
                        &recipe.prompt_template,
                        &variables.cloned(),
                        &slot_images.cloned(),
                        recipe.refs.len(),
                    ).prompt}
                }
            }
            div { class: "run-bar-foot",
                if input_id.cloned().is_empty() {
                    span { class: "hint", "新输入 · 生成时自动保存" }
                } else {
                    span { class: "hint", "正在编辑已保存的输入 · 更新自动保存" }
                }
                div { class: "run-bar-actions",
                    button {
                        class: "ghost-btn small",
                        title: "清空为新的输入",
                        onclick: move |_| {
                            input_id.set(String::new());
                            variables.set(Default::default());
                            slot_images.set(Default::default());
                        },
                        "新空白"
                    }
                    button {
                        class: "btn primary",
                        onclick: generate,
                        disabled: recipe.prompt_template.trim().is_empty(),
                        crate::ui::icons::IconSparkles { size: 14 }
                        "生成"
                        kbd { "⌘↩" }
                    }
                }
            }
        }
    }
}

fn run_bar_state() -> (
    Signal<BTreeMap<String, String>>,
    Signal<BTreeMap<String, AssetRef>>,
    Signal<String>,
) {
    (
        use_signal(BTreeMap::new),
        use_signal(BTreeMap::new),
        use_signal(String::new),
    )
}

/// 图片槽 chip：绑定 / 预览 / 清除 / 上传
#[component]
fn SlotImageChip(
    state: AppState,
    slot: String,
    bound: Option<AssetRef>,
    onbind: EventHandler<AssetRef>,
    onclear: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: if bound.is_some() { "slot-image bound" } else { "slot-image" },
            div { class: "slot-image-head",
                span { class: "param-label slot-name", "{{img:{slot}}}" }
                if bound.is_some() {
                    button {
                        class: "icon-btn",
                        title: "移除",
                        onclick: move |e| {
                            e.stop_propagation();
                            onclear(());
                        },
                        crate::ui::icons::IconX { size: 10 }
                    }
                }
            }
            if let Some(asset) = &bound {
                img {
                    class: "slot-thumb",
                    src: "{asset.url()}",
                    onclick: move |e| {
                        e.stop_propagation();
                    },
                }
            } else {
                label { class: "slot-upload",
                    crate::ui::icons::IconImage { size: 15 }
                    input {
                        r#type: "file",
                        accept: "image/*",
                        style: "display:none",
                        onchange: move |e| {
                            let files = e.files();
                            spawn(async move {
                                if let Some(file) = files.into_iter().next() {
                                    let name = file.name();
                                    match file.read_bytes().await {
                                        Ok(bytes) => match upload_asset(bytes.to_vec(), name).await {
                                            Ok(asset) => onbind.call(asset),
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

// ================= 模型选择 / 参考图 / Mask =================

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

fn upload_refs_and_update(state: AppState, mut draft: Signal<Recipe>, e: Event<FormData>) {
    let files = e.files();
    spawn(async move {
        for file in files {
            let filename = file.name();
            match file.read_bytes().await {
                Ok(bytes) => match upload_asset(bytes.to_vec(), filename).await {
                    Ok(asset) => {
                        draft.with_mut(|r| r.refs.push(asset));
                    }
                    Err(err) => state.toast(format!("上传失败：{err}"), "error"),
                },
                Err(e) => state.toast(format!("读取文件失败：{e}"), "error"),
            }
        }
    });
}

#[component]
fn RefsStrip(state: AppState, mut draft: Signal<Recipe>) -> Element {
    let refs = draft.cloned().refs;
    rsx! {
        div { class: "ref-strip",
            div { class: "ref-thumbs",
                for r in refs.iter().cloned() {
                    div { key: "{r.id}", class: "ref-thumb",
                        img { src: "{r.url()}", loading: "lazy" }
                        button {
                            class: "ref-thumb-remove",
                            title: "移除",
                            onclick: move |_| {
                                let rid = r.id.clone();
                                draft.with_mut(|x| x.refs.retain(|x| x.id != rid));
                            },
                            crate::ui::icons::IconX { size: 10 }
                        }
                    }
                }
                label { class: "ref-add",
                    title: "上传固定参考图",
                    crate::ui::icons::IconImage { size: 15 }
                    span { "参考图" }
                    input {
                        r#type: "file",
                        accept: "image/*",
                        multiple: true,
                        style: "display:none",
                        onchange: move |e| upload_refs_and_update(state, draft, e),
                    }
                }
            }
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
