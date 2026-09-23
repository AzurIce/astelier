//! 输入准备区：一份输入 = 模板的一次具体化 ——
//! 文字槽 / 图片槽变量 + 额外参考图 + mask 覆盖 + 参数覆盖，
//! 下方实时预览合并后的最终请求，点「生成」触发批次（生成时自动保存输入）。

use crate::api::*;
use crate::app::AppState;
use crate::model::{merge_request, AssetRef, MaskOverride, ParamMap, ParamValue, Recipe, RecipeInput};
use crate::ui::params::OverrideParamsRow;
use crate::ui::widgets::{Segment, Segmented};
use dioxus::prelude::*;
use std::collections::BTreeMap;

/// 输入草稿：显式保存，避免每次击键落库 / 版本 +1。
#[derive(Clone, Default, PartialEq)]
struct InputDraft {
    /// 空 = 新输入草稿（生成/保存时自动落库）
    input_id: String,
    title: Option<String>,
    variables: BTreeMap<String, String>,
    images: BTreeMap<String, AssetRef>,
    extra_refs: Vec<AssetRef>,
    mask_override: Option<MaskOverride>,
    param_overrides: ParamMap,
}

impl InputDraft {
    fn blank() -> Self {
        Self::default()
    }

    fn from_input(input: &RecipeInput) -> Self {
        Self {
            input_id: input.id.clone(),
            title: input.title.clone(),
            variables: input.variables.clone(),
            images: input.images.clone(),
            extra_refs: input.extra_refs.clone(),
            mask_override: input.mask_override.clone(),
            param_overrides: input.param_overrides.clone(),
        }
    }

    fn to_input(&self, recipe_id: &str) -> RecipeInput {
        RecipeInput {
            id: self.input_id.clone(),
            recipe_id: recipe_id.to_string(),
            title: self.title.clone(),
            variables: self.variables.clone(),
            images: self.images.clone(),
            extra_refs: self.extra_refs.clone(),
            mask_override: self.mask_override.clone(),
            param_overrides: self.param_overrides.clone(),
            version: 1, // 占位；版本由服务端权威管理
            created_at: 0,
            updated_at: 0,
        }
    }

    fn is_blank(&self) -> bool {
        self.title.is_none()
            && self.variables.values().all(|v| v.trim().is_empty())
            && self.images.is_empty()
            && self.extra_refs.is_empty()
            && self.mask_override.is_none()
            && self.param_overrides.is_empty()
    }
}

#[component]
pub fn InputBand(state: AppState, recipe: Recipe) -> Element {
    let mut show_advanced = state.show_advanced;
    let mut draft = use_signal(InputDraft::blank);
    let mut loaded_for = use_signal(String::new);
    // mask 分段控件的临时态：选「自定义」但尚未上传时不写入草稿
    let mut mask_seg = use_signal(|| "inherit".to_string());

    // 选中输入变化 → 载入草稿；未选中 → 空白草稿。
    // loaded_for 守卫：列表刷新（保存后回灌）不覆盖正在编辑的草稿。
    use_effect(move || {
        let iid = state.selected_input_id();
        if loaded_for.cloned() == iid {
            return;
        }
        loaded_for.set(iid.clone());
        let stored = state
            .recipe_inputs()
            .into_iter()
            .find(|i| i.id == iid && i.recipe_id == state.selected_recipe_id());
        match stored {
            Some(input) => {
                draft.set(InputDraft::from_input(&input));
                mask_seg.set(match &input.mask_override {
                    Some(MaskOverride::Off) => "off".into(),
                    Some(MaskOverride::Custom(_)) => "custom".into(),
                    None => "inherit".into(),
                });
            }
            None => {
                draft.set(InputDraft::blank());
                mask_seg.set("inherit".into());
            }
        }
    });

    let saved_input = state.selected_input();
    let d = draft.cloned();
    let dirty = match &saved_input {
        Some(s) if s.id == d.input_id && !d.input_id.is_empty() => {
            !s.same_content(&d.to_input(&recipe.id)) || s.title != d.title
        }
        _ => !d.is_blank(),
    };
    let version_now = saved_input.as_ref().map(|s| s.version).unwrap_or(0);

    let profile = state.profile_of(&recipe.model_id);
    let (vars, img_slots) = crate::model::template_variables(&recipe.prompt_template);

    // ---- 生成 ----
    let recipe_gen = recipe.clone();
    let generate = move |_| {
        let r = recipe_gen.clone();
        let d = draft.cloned();
        let input = d.to_input(&r.id);
        // 前端先校验（服务端 resolve 时还会权威校验）
        if let Err(msg) = crate::model::resolve_request(&r, &input) {
            state.toast(msg, "error");
            return;
        }
        if let Some(profile) = state.profile_of(&r.model_id) {
            let resolved = merge_request(&r, &input);
            if let Err(msg) = crate::model::validate_request(
                &profile,
                &r.model_id,
                &resolved.params,
                resolved.images.len(),
            ) {
                state.toast(msg, "error");
                return;
            }
        }
        let mut state = state;
        let mut draft = draft;
        spawn(async move {
            match start_run(r.id.clone(), input).await {
                Ok(run) => {
                    if let Some(new_iid) = run.input_id.clone() {
                        // 先把选中与草稿基准都指向该输入（在 await 之前完成，
                        // 防止列表回灌触发载入效果清空草稿）
                        draft.with_mut(|d| d.input_id = new_iid.clone());
                        loaded_for.set(new_iid.clone());
                        state.selected_input.set(new_iid);
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

    // ---- 保存输入 ----
    let recipe_save = recipe.clone();
    let save = move |_| {
        let r = recipe_save.clone();
        let d = draft.cloned();
        let input = d.to_input(&r.id);
        let mut state = state;
        let mut draft = draft;
        spawn(async move {
            match update_input(input).await {
                Ok(saved) => {
                    draft.with_mut(|d| d.input_id = saved.id.clone());
                    loaded_for.set(saved.id.clone());
                    state.selected_input.set(saved.id);
                    if let Ok(list) = list_inputs(r.id.clone()).await {
                        state.recipe_inputs.set(list);
                    }
                    state.toast("输入已保存", "ok");
                }
                Err(e) => state.toast(format!("保存失败：{e}"), "error"),
            }
        });
    };

    // ---- 新空白 ----
    let new_blank = move |_| {
        state.selected_input.set(String::new());
        draft.set(InputDraft::blank());
        mask_seg.set("inherit".into());
    };

    let mask_seg_value = mask_seg.cloned();
    let recipe_mask = recipe.mask.clone();

    rsx! {
        div { class: "input-band",
            div { class: "input-band-head",
                span { class: "section-label", "输入" }
                input {
                    class: "text-input input-title",
                    r#type: "text",
                    value: "{d.title.clone().unwrap_or_default()}",
                    placeholder: "标题（留空自动推导）",
                    oninput: move |e| {
                        let v = e.value();
                        let v = if v.trim().is_empty() { None } else { Some(v) };
                        draft.with_mut(|d| d.title = v);
                    },
                }
                if d.input_id.is_empty() {
                    span { class: "badge", "新输入" }
                } else if dirty {
                    span { class: "dirty-chip", "未保存修改 · 保存后 v{version_now} → v{version_now + 1}" }
                } else {
                    span { class: "version-chip", "v{version_now}" }
                }
            }

            div { class: "input-band-body",
                if vars.is_empty() && img_slots.is_empty() {
                    div { class: "hint input-band-hint",
                        "模板里还没有槽位 — 在上方展开模板，写 {{文字槽}} 或 {{img:图片槽}} 后，这里会出现对应的输入框。"
                    }
                }

                // ---- 文字槽 ----
                for (name, val_now) in vars.iter().map(|n| {
                    let val = d.variables.get(n).cloned().unwrap_or_default();
                    (n.clone(), val)
                }) {
                    div { class: "slot-field",
                        span { class: "param-label slot-name", "{{{name}}}" }
                        input {
                            class: "text-input slot-input",
                            r#type: "text",
                            value: "{val_now}",
                            placeholder: "{name} 的值",
                            oninput: move |e| {
                                let v = e.value();
                                draft.with_mut(|d| {
                                    d.variables.insert(name.clone(), v);
                                });
                            },
                        }
                    }
                }

                // ---- 图片槽 ----
                if !img_slots.is_empty() {
                    div { class: "slot-row",
                        for slot in img_slots.iter().cloned() {
                            {
                                let slot_bind = slot.clone();
                                let slot_clear = slot.clone();
                                rsx! {
                                    SlotImageChip {
                                        state,
                                        slot: slot.clone(),
                                        bound: d.images.get(&slot).cloned(),
                                        onbind: move |asset: AssetRef| {
                                            draft.with_mut(|d| {
                                                d.images.insert(slot_bind.clone(), asset);
                                            });
                                        },
                                        onclear: move |_| {
                                            draft.with_mut(|d| {
                                                d.images.remove(&slot_clear);
                                            });
                                        },
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- 额外参考图 ----
                label { class: "field",
                    span { class: "field-label", "额外参考图（跟输入走 · 排在配方固定图之后）" }
                    crate::ui::widgets::RefsStrip {
                        state,
                        refs: d.extra_refs.clone(),
                        on_added: move |asset: AssetRef| {
                            draft.with_mut(|d| d.extra_refs.push(asset));
                        },
                        on_removed: move |id: String| {
                            draft.with_mut(|d| d.extra_refs.retain(|x| x.id != id));
                        },
                    }
                }

                // ---- Mask 覆盖 ----
                label { class: "field",
                    span { class: "field-label", "Mask" }
                    div { class: "mask-override-row",
                        Segmented {
                            segments: vec![
                                Segment { value: "inherit".into(), label: "跟随配方".into(), title: Some("使用配方设置的 mask".into()) },
                                Segment { value: "off".into(), label: "不使用".into(), title: Some("本次输入显式不用 mask".into()) },
                                Segment { value: "custom".into(), label: "自定义".into(), title: Some("为此输入上传 mask".into()) },
                            ],
                            value: mask_seg_value.clone(),
                            compact: true,
                            onpick: move |v: String| {
                                mask_seg.set(v.clone());
                                draft.with_mut(|d| {
                                    d.mask_override = match v.as_str() {
                                        "off" => Some(MaskOverride::Off),
                                        "custom" => {
                                            // 保留已有的自定义 mask；未上传时不改写草稿
                                            match &d.mask_override {
                                                Some(m @ MaskOverride::Custom(_)) => Some(m.clone()),
                                                _ => None,
                                            }
                                        }
                                        _ => None,
                                    };
                                });
                            },
                        }
                        if mask_seg_value == "inherit" {
                            if let Some(m) = &recipe_mask {
                                img { class: "mask-preview", title: "配方的 mask", src: "{m.url()}" }
                            } else {
                                span { class: "hint", "配方未设置 mask" }
                            }
                        } else if mask_seg_value == "off" {
                            span { class: "hint", "本次将不发送 mask" }
                        } else {
                            MaskCustomUpload { state, draft }
                        }
                    }
                }

                // ---- 参数覆盖 ----
                if let Some(p) = profile.clone() {
                    div { class: "field",
                        span { class: "field-label", "参数覆盖（留空 = 继承配方）" }
                        OverrideParamsRow {
                            profile: p.clone(),
                            base: recipe.params.clone(),
                            overrides: d.param_overrides.clone(),
                            defs: p.params_for(crate::model::Mode::Gen, false).into_iter().cloned().collect(),
                            on_override: move |(key, val): (String, ParamValue)| {
                                draft.with_mut(|d| {
                                    d.param_overrides.insert(key, val);
                                });
                            },
                            on_revert: move |key: String| {
                                draft.with_mut(|d| {
                                    d.param_overrides.remove(&key);
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
                            OverrideParamsRow {
                                profile: p.clone(),
                                base: recipe.params.clone(),
                                overrides: d.param_overrides.clone(),
                                defs: p.params_for(crate::model::Mode::Gen, true).into_iter().cloned().collect(),
                                on_override: move |(key, val): (String, ParamValue)| {
                                    draft.with_mut(|d| {
                                        d.param_overrides.insert(key, val);
                                    });
                                },
                                on_revert: move |key: String| {
                                    draft.with_mut(|d| {
                                        d.param_overrides.remove(&key);
                                    });
                                },
                            }
                        }
                    }
                }

                // ---- 最终请求预览 ----
                RequestPreview { recipe: recipe.clone(), draft: d.clone() }
            }

            div { class: "input-band-foot",
                if d.input_id.is_empty() {
                    span { class: "hint", "新输入 · 生成时自动保存" }
                } else {
                    span { class: "hint", "输入 v{version_now}" }
                }
                div { class: "input-band-actions",
                    button {
                        class: "ghost-btn small",
                        title: "清空为新的输入",
                        onclick: new_blank,
                        "新空白"
                    }
                    if dirty {
                        button {
                            class: "btn small",
                            onclick: save,
                            "保存输入"
                        }
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

/// 最终请求预览：合并后的 prompt + 发送图片序列（标注来源与顺序）+ 生效 mask。
#[component]
fn RequestPreview(recipe: Recipe, draft: InputDraft) -> Element {
    let resolved = merge_request(&recipe, &draft.to_input(&recipe.id));
    let n_recipe = recipe.refs.len();
    let n_extra = draft.extra_refs.len();

    rsx! {
        div { class: "request-preview",
            div { class: "preview-label",
                crate::ui::icons::IconSparkles { size: 11 }
                "最终请求预览"
            }
            if resolved.prompt.trim().is_empty() {
                span { class: "hint", "模板为空" }
            } else {
                pre { class: "preview-prompt", "{resolved.prompt}" }
            }
            if !resolved.images.is_empty() {
                div { class: "preview-seq",
                    for (idx, asset) in resolved.images.iter().enumerate() {
                        div { key: "{asset.id}", class: "seq-item",
                            img { src: "{asset.url()}", loading: "lazy" }
                            span { class: "seq-num", "{idx + 1}" }
                            span {
                                class: if idx < n_recipe {
                                    "seq-src src-recipe"
                                } else if idx < n_recipe + n_extra {
                                    "seq-src src-input"
                                } else {
                                    "seq-src src-slot"
                                },
                                if idx < n_recipe { "配" } else if idx < n_recipe + n_extra { "输" } else { "槽" }
                            }
                        }
                    }
                    if let Some(m) = &resolved.mask {
                        div { class: "seq-item seq-mask", title: "生效 mask",
                            img { src: "{m.url()}", loading: "lazy" }
                            crate::ui::icons::IconMask { size: 11 }
                        }
                    }
                }
            }
            span { class: "hint seq-note", "发送顺序：配方固定图（配）→ 输入额外图（输）→ 槽位图（槽）" }
        }
    }
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

/// 自定义 mask 上传（输入层覆盖）
#[component]
fn MaskCustomUpload(state: AppState, mut draft: Signal<InputDraft>) -> Element {
    let custom = draft.cloned().mask_override;
    let asset = match custom {
        Some(MaskOverride::Custom(a)) => Some(a),
        _ => None,
    };
    rsx! {
        div { class: "mask-row",
            if let Some(m) = asset {
                div { class: "ref-mask-chip",
                    crate::ui::icons::IconMask { size: 13 }
                    span { "mask" }
                    img { class: "mask-preview", src: "{m.url()}" }
                    button {
                        class: "icon-btn",
                        title: "移除自定义 mask（回到跟随配方）",
                        onclick: move |_| {
                            draft.with_mut(|d| d.mask_override = None);
                        },
                        crate::ui::icons::IconX { size: 11 }
                    }
                }
            } else {
                label { class: "ref-add mask",
                    title: "上传 mask PNG（透明区域 = 重绘区域）",
                    crate::ui::icons::IconMask { size: 15 }
                    span { "上传 Mask" }
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
                                            Ok(asset) => {
                                                draft.with_mut(|d| d.mask_override = Some(MaskOverride::Custom(asset)));
                                            }
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
