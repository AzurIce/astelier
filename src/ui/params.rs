//! 参数控件：由 ModelProfile 的 params schema 驱动渲染。
//!
//! select → 分段控件（≤5 段）或下拉；number → 步进器；
//! size → 宽高比 + 目标百万像素（ComfyUI Resolution Selector 风格）；
//! text → 行内输入。首个「默认」段 = Unset = 不随请求发送。

use crate::model::{ModelProfile, ParamDef, ParamKind, ParamValue, SizeRule};
use crate::ui::widgets::{Dropdown, DropdownItem, Segment, Segmented, Stepper};
use dioxus::prelude::*;

fn option_label(key: &str, opt: &str) -> String {
    match (key, opt) {
        ("quality", "auto") => "自动",
        ("quality", "high") => "高",
        ("quality", "medium") => "中",
        ("quality", "low") => "低",
        ("quality", "xhigh") => "超清",
        ("quality", "max") => "极致",
        ("background", "transparent") => "透明",
        ("background", "opaque") => "不透明",
        ("moderation", "low") => "宽松",
        ("moderation", "auto") => "标准",
        ("input_fidelity", "high") => "高保真",
        ("input_fidelity", "low") => "低保真",
        _ => opt,
    }
    .into()
}

fn preset_label(preset: &str) -> (String, Option<String>) {
    match preset {
        "auto" => ("自动".into(), Some("由模型决定".into())),
        "1024x1024" => ("1:1".into(), Some("1024×1024".into())),
        "1536x1024" => ("3:2".into(), Some("1536×1024".into())),
        "1024x1536" => ("2:3".into(), Some("1024×1536".into())),
        other => (other.into(), None),
    }
}

/// 单个参数控件（按 kind 分发），onset 输出 (统一键, 新值)
#[component]
fn ParamItem(
    profile: ModelProfile,
    params: std::collections::BTreeMap<String, ParamValue>,
    def: ParamDef,
    onset: EventHandler<(String, ParamValue)>,
) -> Element {

    let current = params.get(&def.key).cloned().unwrap_or(ParamValue::Unset);

    let control = match def.kind {
        ParamKind::Select => {
            let cur_text = match &current {
                ParamValue::Text(s) => Some(s.clone()),
                _ => None,
            };
            let segments: Vec<Segment> = std::iter::once(Segment {
                value: "__unset__".into(),
                label: "默认".into(),
                title: Some("不发送该参数".into()),
            })
            .chain(
                def.options
                    .iter()
                    .map(|o| Segment {
                        value: o.clone(),
                        label: option_label(&def.key, o),
                        title: Some(o.clone()),
                    }),
            )
            .collect();
            let seg_value = cur_text.unwrap_or_else(|| "__unset__".into());
            let key = def.key.clone();
            if segments.len() <= 5 {
                rsx! {
                    Segmented {
                        segments,
                        value: seg_value,
                        compact: false,
                        onpick: move |v: String| {
                            let val = if v == "__unset__" {
                                ParamValue::Unset
                            } else {
                                ParamValue::Text(v)
                            };
                            onset.call((key.clone(), val));
                        },
                    }
                }
            } else {
                rsx! {
                    select {
                        class: "select",
                        value: seg_value,
                        onchange: move |e| {
                            let v = e.value();
                            let val = if v == "__unset__" {
                                ParamValue::Unset
                            } else {
                                ParamValue::Text(v)
                            };
                            onset.call((key.clone(), val));
                        },
                        option { value: "__unset__", "默认" }
                        for o in def.options.iter() {
                            option { value: "{o}", "{option_label(&def.key, o)}" }
                        }
                    }
                }
            }
        }
        ParamKind::Number => {
            let current_n = params.get(&def.key).and_then(|v| match v {
                ParamValue::Number(n) => Some(*n),
                _ => None,
            });
            let key = def.key.clone();
            rsx! {
                Stepper {
                    value: current_n,
                    min: def.min.unwrap_or(0.0),
                    max: def.max.unwrap_or(99.0),
                    step: 1.0,
                    onchange: move |v: Option<f64>| {
                        onset.call((
                            key.clone(),
                            v.map(ParamValue::Number).unwrap_or(ParamValue::Unset),
                        ));
                    },
                }
            }
        }
        ParamKind::Text => {
            let current_t = params.get(&def.key).and_then(|v| match v {
                ParamValue::Text(s) => Some(s.clone()),
                _ => None,
            }).unwrap_or_default();
            let key = def.key.clone();
            rsx! {
                input {
                    class: "text-input",
                    r#type: "text",
                    value: "{current_t}",
                    placeholder: "留空不发送",
                    spellcheck: "false",
                    oninput: move |e| {
                        let v = e.value();
                        onset.call((key.clone(), ParamValue::Text(v)));
                    },
                }
            }
        }
        ParamKind::Size => {
            let current_size = params.get(&def.key).and_then(|v| match v {
                ParamValue::Size(s) => Some(s.clone()),
                _ => None,
            });
            if !profile.size_ratios.is_empty() {
                // ---- 宽高比 + 百万像素 → W×H ----
                // 从存储值反推 (比例, MP)；比例偏差 >5% 视为自定义
                let found: Option<(usize, f64)> = current_size.as_deref().and_then(|s| {
                    let (w, h) = s.split_once('x')?;
                    let (w, h) = (w.trim().parse::<f64>().ok()?, h.trim().parse::<f64>().ok()?);
                    if w <= 0.0 || h <= 0.0 {
                        return None;
                    }
                    let r = w / h;
                    let (ri, preset) = profile.size_ratios.iter().enumerate().min_by(|(_, a), (_, b)| {
                        let da = ((a.w as f64 / a.h as f64) - r).abs();
                        let db = ((b.w as f64 / b.h as f64) - r).abs();
                        da.partial_cmp(&db).unwrap()
                    })?;
                    let target = preset.w as f64 / preset.h as f64;
                    if ((target - r) / target).abs() > 0.05 {
                        return None;
                    }
                    Some((ri, w * h / (1024.0 * 1024.0)))
                });
                let is_custom = found.is_none() && current_size.is_some();
                let mp_now = found.as_ref().map(|(_, m)| *m).unwrap_or(1.0);
                let ratio_idx_now = found.map(|(r, _)| r).unwrap_or(0);

                let mut ratio_items: Vec<DropdownItem> = vec![DropdownItem {
                    value: "__unset__".into(),
                    label: "自动".into(),
                    caption: Some("不发送 size，由模型决定".into()),
                    badges: vec![],
                }];
                ratio_items.extend(profile.size_ratios.iter().map(|r| {
                    let hint = if r.w == r.h {
                        "方形"
                    } else if r.w > r.h {
                        "横"
                    } else {
                        "竖"
                    };
                    DropdownItem {
                        value: r.label.clone(),
                        label: r.label.clone(),
                        caption: Some(hint.into()),
                        badges: vec![],
                    }
                }));
                ratio_items.push(DropdownItem {
                    value: "__custom__".into(),
                    label: "自定义 W × H".into(),
                    caption: Some("手动输入分辨率".into()),
                    badges: vec![],
                });
                let ratio_active = if is_custom {
                    "__custom__".to_string()
                } else {
                    found
                        .map(|(r, _)| profile.size_ratios[r].label.clone())
                        .unwrap_or_else(|| "__unset__".into())
                };
                let ratio_label = if is_custom {
                    "自定义 W × H".to_string()
                } else if let Some((r, _)) = found {
                    profile.size_ratios[r].label.clone()
                } else {
                    "自动".to_string()
                };

                let (cw, ch) = profile
                    .size_ratios
                    .get(ratio_idx_now)
                    .map(|r| profile.compute_mp_size(r.w, r.h, mp_now))
                    .unwrap_or((1024, 1024));

                let key_dd = def.key.clone();
                let key_mp = def.key.clone();
                let profile_dd = profile.clone();
                let profile_mp = profile.clone();
                let cur_pick = current_size.clone();
                let mp_dd = mp_now;

                rsx! {
                    div { class: "size-row",
                        Dropdown {
                            class: "size-dropdown",
                            label: ratio_label,
                            items: ratio_items,
                            value: ratio_active,
                            disabled: false,
                            drop_up: true,
                            onpick: move |v: String| {
                                let val = match v.as_str() {
                                    "__unset__" => ParamValue::Unset,
                                    "__custom__" => ParamValue::Size(
                                        cur_pick.clone().unwrap_or_else(|| "1024x1024".into()),
                                    ),
                                    label => {
                                        let Some(r) = profile_dd
                                            .size_ratios
                                            .iter()
                                            .find(|x| x.label == label)
                                        else {
                                            return;
                                        };
                                        let (w, h) = profile_dd.compute_mp_size(r.w, r.h, mp_dd);
                                        ParamValue::Size(format!("{w}x{h}"))
                                    }
                                };
                                onset.call((key_dd.clone(), val));
                            },
                        }
                        div { class: "param",
                            span {
                                class: "param-label",
                                title: "目标总百万像素，1.0 MP ≈ 1024×1024",
                                "像素"
                            }
                            Stepper {
                                value: Some(mp_now),
                                min: 0.25,
                                max: 16.0,
                                step: 0.5,
                                onchange: move |v: Option<f64>| {
                                    let mp = v.unwrap_or(1.0);
                                    if let Some(r) = profile_mp.size_ratios.get(ratio_idx_now) {
                                        let (w, h) = profile_mp.compute_mp_size(r.w, r.h, mp);
                                        onset.call((key_mp.clone(), ParamValue::Size(format!("{w}x{h}"))));
                                    }
                                },
                            }
                        }
                        span { class: "size-computed", "= {cw} × {ch}" }
                        if is_custom {
                            SizeCustom {
                                rule: profile.size_rule.clone().unwrap_or_else(SizeRule::gpt_image),
                                value: current_size.clone().unwrap_or_else(|| "1024x1024".into()),
                                onset,
                            }
                        }
                    }
                }
            } else {
                // ---- 扁平预设（无比例轴的模型）----
                let mut segments: Vec<Segment> = vec![Segment {
                    value: "__unset__".into(),
                    label: "默认".into(),
                    title: Some("不发送该参数".into()),
                }];
                segments.extend(def.options.iter().map(|o| {
                    let (label, title) = preset_label(o);
                    Segment {
                        value: o.clone(),
                        label,
                        title,
                    }
                }));
                let has_custom = profile.size_rule.is_some();
                if has_custom {
                    segments.push(Segment {
                        value: "__custom__".into(),
                        label: "自定义".into(),
                        title: None,
                    });
                }
                let seg_value = match &current_size {
                    None => "__unset__".to_string(),
                    Some(s) => {
                        if segments.iter().any(|seg| &seg.value == s) {
                            s.clone()
                        } else if has_custom {
                            "__custom__".to_string()
                        } else {
                            "__unset__".to_string()
                        }
                    }
                };
                let key = def.key.clone();
                let cur_for_pick = current_size.clone();
                let is_custom_size = current_size
                    .as_ref()
                    .map(|s| !segments.iter().any(|seg| &seg.value == s))
                    .unwrap_or(false)
                    && has_custom;
                rsx! {
                    div { class: "size-param",
                        Segmented {
                            segments,
                            value: seg_value,
                            compact: false,
                            onpick: move |v: String| {
                                let val = match v.as_str() {
                                    "__unset__" => ParamValue::Unset,
                                    "__custom__" => ParamValue::Size(
                                        cur_for_pick.clone().unwrap_or_else(|| "1536x1024".into()),
                                    ),
                                    _ => ParamValue::Size(v),
                                };
                                onset.call((key.clone(), val));
                            },
                        }
                        if is_custom_size {
                            SizeCustom {
                                rule: profile.size_rule.clone().unwrap_or_else(SizeRule::gpt_image),
                                value: current_size.clone().unwrap_or_else(|| "1536x1024".into()),
                                onset,
                            }
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div { class: "param",
            span { class: "param-label", title: "{def.api_key()}", "{def.label}" }
            {control}
        }
    }
}

/// 分组渲染顺序：主参数（无标签）→ 输出 → 审核与标识
const GROUP_ORDER: &[&str] = &["", "output", "safety"];

fn group_label(group: &str) -> Option<&'static str> {
    match group {
        "" => None,
        "output" => Some("输出"),
        "safety" => Some("审核与标识"),
        _ => None,
    }
}

/// 单个参数控件（按 kind 分发），onset 输出 (统一键, 新值)
#[component]
pub fn ParamsRow(
    profile: ModelProfile,
    params: std::collections::BTreeMap<String, ParamValue>,
    defs: Vec<ParamDef>,
    hide_groups: Vec<String>,
    onset: EventHandler<(String, ParamValue)>,
) -> Element {
    // 按语义分组（组间按 GROUP_ORDER，组内保持档案定义顺序）
    let mut clusters: Vec<(String, Vec<ParamDef>)> = Vec::new();
    for g in GROUP_ORDER {
        let items: Vec<ParamDef> = defs
            .iter()
            .filter(|d| d.group == *g)
            .cloned()
            .collect();
        if !items.is_empty() {
            clusters.push((g.to_string(), items));
        }
    }
    let rest: Vec<ParamDef> = defs
        .iter()
        .filter(|d| !GROUP_ORDER.contains(&d.group.as_str()))
        .cloned()
        .collect();
    if !rest.is_empty() {
        clusters.push(("__other__".into(), rest));
    }

    rsx! {
        for (group, items) in clusters.iter() {
            div { key: "{group}-{items.len()}", class: "param-group",
                if let Some(label) = group_label(group).filter(|l| !hide_groups.iter().any(|g| g == *l || g == group)) {
                    span { class: "param-group-label", "{label}" }
                }
                div { class: "param-group-items",
                    for def in items.iter() {
                        ParamItem {
                            key: "{def.key}",
                            profile: profile.clone(),
                            params: params.clone(),
                            def: def.clone(),
                            onset,
                        }
                    }
                }
            }
        }
    }
}

/// 自定义 W×H 输入 + SizeRule 实时校验
#[component]
fn SizeCustom(
    rule: SizeRule,
    value: String,
    onset: EventHandler<(String, ParamValue)>,
) -> Element {
    let init = match value.split_once('x') {
        Some((a, b)) => (
            a.trim().parse::<u32>().unwrap_or(1536),
            b.trim().parse::<u32>().unwrap_or(1024),
        ),
        None => (1536, 1024),
    };
    let mut local_w = use_signal(|| init.0.to_string());
    let mut local_h = use_signal(|| init.1.to_string());

    let (err, ok_text) = match (local_w().trim().parse::<u32>(), local_h().trim().parse::<u32>()) {
        (Ok(w), Ok(h)) => match rule.validate(w as i64, h as i64) {
            Err(e) => (Some(e), String::new()),
            Ok(()) => (None, format!("✓ {w}×{h} 可用")),
        },
        _ => (Some("宽高必须是整数".into()), String::new()),
    };

    let emit = move |w: Option<u32>, h: Option<u32>| {
        if let (Some(w), Some(h)) = (w, h) {
            onset.call(("size".into(), ParamValue::Size(format!("{w}x{h}"))));
        }
    };

    rsx! {
        div { class: "size-custom",
            div { class: "size-custom-inputs",
                span { class: "size-x", "W" }
                input {
                    class: "text-input size-w",
                    r#type: "number",
                    min: "{rule.min_side}",
                    max: "{rule.max_w}",
                    step: "{rule.step}",
                    value: "{local_w}",
                    oninput: move |e| {
                        local_w.set(e.value());
                        let w = e.value();
                        let h = local_h();
                        emit(
                            w.trim().parse::<u32>().ok(),
                            h.trim().parse::<u32>().ok(),
                        );
                    },
                }
                span { class: "size-x", "×" }
                input {
                    class: "text-input size-h",
                    r#type: "number",
                    min: "{rule.min_side}",
                    max: "{rule.max_h}",
                    step: "{rule.step}",
                    value: "{local_h}",
                    oninput: move |e| {
                        local_h.set(e.value());
                        let w = local_w();
                        let h = e.value();
                        emit(
                            w.trim().parse::<u32>().ok(),
                            h.trim().parse::<u32>().ok(),
                        );
                    },
                }
            }
            if let Some(err) = err {
                span { class: "size-rule err", "✗ {err}" }
            } else {
                span { class: "size-rule", "{ok_text} · {rule.note}" }
            }
        }
    }
}

/// 输入层参数覆盖区：控件显示「配方值 ⊕ 覆盖」的合并结果。
/// 未覆盖项首次交互即产生覆盖；已覆盖项高亮并可一键还原为继承。
#[component]
pub fn OverrideParamsRow(
    profile: ModelProfile,
    /// 配方当前参数（继承基准）
    base: std::collections::BTreeMap<String, ParamValue>,
    /// 输入覆盖项（只存被显式覆盖的键）
    overrides: std::collections::BTreeMap<String, ParamValue>,
    defs: Vec<ParamDef>,
    on_override: EventHandler<(String, ParamValue)>,
    on_revert: EventHandler<String>,
) -> Element {
    let mut display = base.clone();
    for (k, v) in &overrides {
        display.insert(k.clone(), v.clone());
    }

    rsx! {
        div { class: "override-params",
            for def in defs.iter() {
                div {
                    key: "{def.key}",
                    class: if overrides.contains_key(&def.key) {
                        "param-ov overridden"
                    } else {
                        "param-ov"
                    },
                    ParamItem {
                        key: "{def.key}",
                        profile: profile.clone(),
                        params: display.clone(),
                        def: def.clone(),
                        onset: on_override,
                    }
                    if overrides.contains_key(&def.key) {
                        div { class: "param-ov-bar",
                            span { class: "ov-chip", "已覆盖" }
                            button {
                                class: "icon-btn",
                                title: "还原为继承配方值",
                                onclick: {
                                    let key = def.key.clone();
                                    move |_| on_revert(key.clone())
                                },
                                crate::ui::icons::IconX { size: 10 }
                            }
                        }
                    }
                }
            }
        }
    }
}
