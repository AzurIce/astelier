//! 设置弹窗：Provider 管理 + 模型元数据覆盖。

use crate::api::*;
use crate::app::AppState;
use crate::model::{Config, Provider};
use crate::ui::widgets::Modal;
use dioxus::prelude::*;

#[component]
pub fn SettingsModal(state: AppState) -> Element {
    let mut editing = use_signal(|| state.config().unwrap_or_default());
    let mut sel_provider = use_signal(|| {
        state
            .config()
            .map(|c| c.active_provider.clone())
            .unwrap_or_default()
    });
    let mut show_key = use_signal(|| false);

    let cfg = editing();
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == sel_provider())
        .cloned();

    // ---- 组件体内构造表单回调（各自持有 provider id 克隆）----
    let pid_field = provider.as_ref().map(|p| p.id.clone());
    let pid_a = pid_field.clone();
    let pid_b = pid_field.clone();
    let pid_c = pid_field.clone();
    let pid_d = pid_field.clone();

    let on_name = move |e: Event<FormData>| {
        let v = e.value();
        if let Some(pid) = &pid_a {
            update_provider(editing, pid, |x| x.name = v.clone());
        }
    };
    let on_url = move |e: Event<FormData>| {
        let v = e.value();
        if let Some(pid) = &pid_b {
            update_provider(editing, pid, |x| x.base_url = v.clone());
        }
    };
    let on_key = move |e: Event<FormData>| {
        let v = e.value();
        if let Some(pid) = &pid_c {
            update_provider(editing, pid, |x| x.api_key = v.clone());
        }
    };
    let on_models = move |e: Event<FormData>| {
        let v = e.value();
        if let Some(pid) = &pid_d {
            update_provider(editing, pid, |x| {
                x.models = v
                    .lines()
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect();
            });
        }
    };
    let p_name = provider
        .as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_default();
    let p_url = provider
        .as_ref()
        .map(|p| p.base_url.clone())
        .unwrap_or_default();
    let p_key = provider
        .as_ref()
        .map(|p| p.api_key.clone())
        .unwrap_or_default();
    let p_models_text = provider
        .as_ref()
        .map(|p| p.models.join("\n"))
        .unwrap_or_default();
    let has_provider = provider.is_some();

    rsx! {
        Modal {
            title: "设置",
            subtitle: "Provider 与模型元数据仅保存在本机 data/ 目录",
            wide: true,
            onclose: move |_| state.show_settings.set(false),

            div { class: "settings-layout",
                // ---- 左栏：provider 列表 ----
                div { class: "settings-side",
                    div { class: "settings-side-head",
                        span { class: "section-label", "Provider" }
                        button {
                            class: "icon-btn",
                            title: "新增 Provider",
                            onclick: move |_| {
                                let id = format!("p-{}", crate::util::now_ms());
                                let mut next = editing();
                                next.providers.push(Provider {
                                    id: id.clone(),
                                    name: "新 Provider".into(),
                                    base_url: String::new(),
                                    api_key: String::new(),
                                    models: vec![],
                                    overrides: Default::default(),
                                });
                                editing.set(next);
                                sel_provider.set(id);
                            },
                            crate::ui::icons::IconPlus { size: 14 }
                        }
                    }
                for p in cfg.providers.iter().cloned() {
                    ProviderRow {
                        key: "{p.id}",
                        state,
                        editing,
                        sel_provider,
                        provider: p.clone(),
                        selected: p.id == sel_provider(),
                    }
                }                }

                // ---- 右栏：编辑区 ----
                div { class: "settings-main",
                    if has_provider {
                        div { class: "field-grid",
                            label { class: "field",
                                span { class: "field-label", "名称" }
                                input {
                                    class: "text-input",
                                    value: "{p_name}",
                                    oninput: on_name,
                                }
                            }
                            label { class: "field",
                                span { class: "field-label", "Base URL（含 /v1）" }
                                input {
                                    class: "text-input",
                                    value: "{p_url}",
                                    placeholder: "https://api.openai.com/v1",
                                    spellcheck: "false",
                                    oninput: on_url,
                                }
                            }
                            label { class: "field",
                                span { class: "field-label",
                                    "API Key"
                                    span { class: "hint", "· 填全大写变量名（如 OPENAI_API_KEY）自动读环境变量" }
                                }
                                div { class: "key-row",
                                    input {
                                        class: "text-input",
                                        r#type: if show_key() { "text" } else { "password" },
                                        value: "{p_key}",
                                        placeholder: "sk-… 或 OPENAI_API_KEY",
                                        autocomplete: "off",
                                        spellcheck: "false",
                                        oninput: on_key,
                                    }
                                    button {
                                        class: "icon-btn",
                                        tabindex: "-1",
                                        onclick: move |_| show_key.toggle(),
                                        if show_key() { crate::ui::icons::IconEyeOff { size: 14 } } else { crate::ui::icons::IconEye { size: 14 } }
                                    }
                                }
                            }
                            label { class: "field",
                                span { class: "field-label", "模型列表（每行一个）" }
                                textarea {
                                    class: "text-area models-area",
                                    rows: "5",
                                    spellcheck: "false",
                                    value: "{p_models_text}",
                                    oninput: on_models,
                                }
                            }
                        }

                        ModelMetaSection {
                            state,
                            editing,
                            provider_id: pid_field.clone().unwrap_or_default(),
                            models: provider.as_ref().map(|p| p.models.clone()).unwrap_or_default(),
                        }

                    } else {
                        div { class: "settings-empty",
                            "选择或新建一个 Provider 开始配置。"
                        }
                    }
                }
            }

            footer { class: "settings-foot",
                span { class: "hint",
                    "保存后立即生效；API Key 只发往对应 Provider。"
                }
                button {
                    class: "btn primary",
                    onclick: move |_| {
                        spawn(async move {
                            let cfg = editing();
                            let mut next = cfg.clone();
                            if next.active_provider.is_empty() {
                                next.active_provider = next.providers.first().map(|p| p.id.clone()).unwrap_or_default();
                            }
                            if !next.providers.iter().any(|p| p.id == next.active_provider) {
                                next.active_provider = next.providers.first().map(|p| p.id.clone()).unwrap_or_default();
                            }
                            if let Err(e) = save_config(next.clone()).await {
                                state.toast(format!("保存失败：{e}"), "error");
                                return;
                            }
                            state.config.set(Some(next));
                            if let Some(c) = state.config() {
                                if let Ok(list) = resolve_profiles(c.active_provider).await {
                                    state.profiles.set(list);
                                }
                            }
                            state.toast("设置已保存", "ok");
                            state.show_settings.set(false);
                        });
                    },
                    "保存"
                }
            }
        }
    }
}

fn update_provider(mut editing: Signal<Config>, id: &str, f: impl FnOnce(&mut Provider)) {
    editing.with_mut(|cfg| {
        if let Some(p) = cfg.providers.iter_mut().find(|p| p.id == id) {
            f(p);
        }
    });
}

/// 模型元数据区：列出当前 provider 的模型
#[component]
fn ModelMetaSection(
    state: AppState,
    editing: Signal<Config>,
    provider_id: String,
    models: Vec<String>,
) -> Element {
    let flags: Vec<(String, bool)> = models
        .iter()
        .map(|m| {
            let ov = editing()
                .providers
                .iter()
                .find(|p| p.id == provider_id)
                .map(|p| p.overrides.contains_key(m))
                .unwrap_or(false);
            (m.clone(), ov)
        })
        .collect();

    rsx! {
        div { class: "meta-section",
            span { class: "section-label", "模型元数据" }
            div { class: "meta-list",
                for (m, overridden) in flags.iter().cloned() {
                    ModelMetaItem {
                        key: "{m}",
                        state,
                        editing,
                        provider_id: provider_id.clone(),
                        model: m.clone(),
                        overridden,
                    }
                }
            }
            if models.is_empty() {
                div { class: "hint", "此 Provider 还没有模型，先在上方模型列表添加。" }
            }
        }
    }
}

static META_TEMPLATES: &[(&str, &str)] = &[
    ("gpt-image-2.5 全能力（xhigh/max · 任意尺寸 · 流式）", "gpt-image-2.5"),
    ("网关通用（prompt / size / quality / n）", "generic"),
    ("Banana / Seedream 风格（结构不同，字段名可改）", "banana"),
];

/// 单个模型：行 + 展开的元数据 JSON 编辑器（open 状态自持）
#[component]
fn ModelMetaItem(
    state: AppState,
    mut editing: Signal<Config>,
    provider_id: String,
    model: String,
    overridden: bool,
) -> Element {
    let mut open = use_signal(|| false);
    let mut draft_json = use_signal(String::new);
    let mut draft_status = use_signal(|| None::<(bool, String)>);
    let mut template = use_signal(|| 0usize);
    let mut was_open = use_signal(|| false);

    // 展开时把当前档案（或已保存覆盖）填入草稿
    let pid_eff = provider_id.clone();
    let m_eff = model.clone();
    use_effect(move || {
        if open() && !was_open() {
            was_open.set(true);
            let cfg = editing();
            let ov = cfg
                .providers
                .iter()
                .find(|p| p.id == pid_eff)
                .and_then(|p| p.overrides.get(&m_eff));
            let json = match ov {
                Some(v) => serde_json::to_string_pretty(v).unwrap_or_default(),
                None => serde_json::to_string_pretty(&crate::profiles::merged(&m_eff, None))
                    .unwrap_or_default(),
            };
            draft_json.set(json);
            draft_status.set(None);
        }
        if !open() {
            was_open.set(false);
        }
    });

    let pid_reset = provider_id.clone();
    let pid_apply = provider_id.clone();
    let m_reset = model.clone();
    let m_apply = model.clone();

    let is_open = open();
    let on_toggle = move |_| open.set(!is_open);

    let on_load_template = move |_| {
        let t = META_TEMPLATES[template()].1;
        let prof = match t {
            "gpt-image-2.5" => crate::profiles::merged("gpt-image-2.5", None),
            "banana" => banana_template(),
            _ => crate::profiles::merged("generic-model", None),
        };
        draft_json.set(serde_json::to_string_pretty(&prof).unwrap_or_default());
    };
    let on_reset = move |_| {
        editing.with_mut(|cfg| {
            if let Some(p) = cfg.providers.iter_mut().find(|p| p.id == pid_reset) {
                p.overrides.remove(&m_reset);
            }
        });
        draft_status.set(Some((true, "已重置为内置档案（记得保存）".into())));
    };
    let on_apply = move |_| {
        match serde_json::from_str::<serde_json::Value>(&draft_json()) {
            Err(e) => draft_status.set(Some((false, format!("JSON 解析失败：{e}")))),
            Ok(v) => match serde_json::from_value::<crate::model::ModelProfile>(v.clone()) {
                Err(e) => draft_status.set(Some((false, format!("档案字段有误：{e}")))),
                Ok(_) => {
                    editing.with_mut(|cfg| {
                        if let Some(p) = cfg.providers.iter_mut().find(|p| p.id == pid_apply) {
                            p.overrides.insert(m_apply.clone(), v);
                        }
                    });
                    draft_status.set(Some((true, "已应用（记得保存）".into())));
                }
            },
        }
    };

    rsx! {
        div { class: "meta-item",
            div { class: "meta-row",
                span { class: "mono meta-name", "{model}" }
                div { class: "meta-badges",
                    for (text, kind) in crate::profiles::badges(&crate::profiles::merged(&model, None))
                        .iter()
                    {
                        span { class: "badge badge-{kind}", "{text}" }
                    }
                    if overridden {
                        span { class: "badge badge-custom", "已覆盖" }
                    }
                }
                button {
                    class: "icon-btn",
                    title: "编辑元数据",
                    onclick: on_toggle,
                    if is_open {
                        crate::ui::icons::IconX { size: 13 }
                    } else {
                        crate::ui::icons::IconSliders { size: 13 }
                    }
                }
            }
            if is_open {
                div { class: "meta-editor",
                    div { class: "meta-editor-toolbar",
                        select {
                            class: "select",
                            value: "{template()}",
                            onchange: move |e| template.set(e.value().parse::<usize>().unwrap_or(0)),
                            for (i, (name, _)) in META_TEMPLATES.iter().enumerate() {
                                option { value: "{i}", "{name}" }
                            }
                        }
                        button {
                            class: "ghost-btn small",
                            onclick: on_load_template,
                            "载入模板"
                        }
                        if overridden {
                            button {
                                class: "ghost-btn small",
                                onclick: on_reset,
                                "重置覆盖"
                            }
                        }
                    }
                    textarea {
                        class: "text-area meta-json",
                        rows: "12",
                        spellcheck: "false",
                        value: "{draft_json()}",
                        oninput: move |e| draft_json.set(e.value()),
                    }
                    div { class: "meta-editor-foot",
                        button {
                            class: "btn primary small",
                            onclick: on_apply,
                            "应用覆盖"
                        }
                        if let Some((ok, msg)) = draft_status() {
                            span { class: if ok { "meta-status ok" } else { "meta-status err" }, "{msg}" }
                        }
                    }
                }
            }
        }
    }
}

fn banana_template() -> crate::model::ModelProfile {
    use crate::model::{ApiKind, ParamDef, ParamKind};
    crate::model::ModelProfile {
        id: "nano-banana".into(),
        label: "示例：Banana 风格网关（image_size / aspect_ratio 分档，无 mask）".into(),
        api: ApiKind::Generic,
        stream: false,
        max_prompt: 32_000,
        max_refs: 4,
        mask_edit: false,
        transparent: false,
        edit_note: "指令式编辑（多参考图，无 mask）".into(),
        params: vec![
            ParamDef {
                key: "image_size".into(),
                api_key: String::new(),
                label: "分辨率档".into(),
                kind: ParamKind::Select,
                options: ["1K", "2K", "4K"].iter().map(|s| s.to_string()).collect(),
                min: None,
                max: None,
                advanced: false,
                group: String::new(),
                modes: vec![],
            },
            ParamDef {
                key: "aspect_ratio".into(),
                api_key: String::new(),
                label: "比例".into(),
                kind: ParamKind::Select,
                options: ["auto", "1:1", "3:4", "2:3", "4:3", "16:9", "9:16"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
                min: None,
                max: None,
                advanced: false,
                group: String::new(),
                modes: vec![],
            },
            ParamDef {
                key: "n".into(),
                api_key: String::new(),
                label: "数量".into(),
                kind: ParamKind::Number,
                options: vec![],
                min: Some(1.0),
                max: Some(4.0),
                advanced: false,
                group: String::new(),
                modes: vec![],
            },
        ],
        size_rule: None,
        size_ratios: vec![],
    }
}

/// Provider 列表行：点击选中；悬浮出现删除按钮，两段式确认
#[component]
fn ProviderRow(
    state: AppState,
    mut editing: Signal<Config>,
    mut sel_provider: Signal<String>,
    provider: Provider,
    selected: bool,
) -> Element {
    let mut confirming = use_signal(|| false);
    let pid = provider.id.clone();
    let pid_sel = pid.clone();
    let pid_del = pid.clone();
    let is_confirming = confirming();

    rsx! {
        div {
            class: if selected { "settings-provider selected" } else { "settings-provider" },
            onclick: move |_| sel_provider.set(pid_sel.clone()),
            span { class: "settings-provider-name", "{provider.name}" }
            span { class: "settings-provider-count", "{provider.models.len()}" }
            div { class: "provider-row-actions",
                if is_confirming {
                    button {
                        class: "btn danger small",
                        title: "再次点击确认删除",
                        onclick: move |_| {
                            let mut next = editing();
                            next.providers.retain(|x| x.id != pid_del);
                            if next.active_provider == pid_del {
                                next.active_provider = next
                                    .providers
                                    .first()
                                    .map(|x| x.id.clone())
                                    .unwrap_or_default();
                            }
                            editing.set(next);
                            sel_provider.set(String::new());
                            confirming.set(false);
                        },
                        "确认"
                    }
                } else {
                    button {
                        class: "icon-btn input-item-action",
                        title: "删除此 Provider",
                        onclick: move |_| confirming.set(true),
                        crate::ui::icons::IconTrash { size: 12 }
                    }
                }
            }
        }
    }
}
