//! 内置模型能力档案。
//!
//! params schema 是参数区的渲染契约：表单控件、能力徽章、请求体收集、
//! 自定义尺寸校验全部由它驱动。接入新模型（seeddream / nano banana…）
//! = 增加一份档案或由 provider override 提供元数据，不改代码。
//! （视图辅助 badges 等：React 前端接回模型选择 UI 前暂无调用方）
#![allow(dead_code)]

use crate::model::{ApiKind, ModelProfile, ParamDef, ParamKind, ParamValue, RatioPreset, SizeRule};

fn param(key: &str, label: &str, kind: ParamKind) -> ParamDef {
    ParamDef {
        key: key.into(),
        api_key: String::new(),
        label: label.into(),
        kind,
        options: vec![],
        min: None,
        max: None,
        advanced: false,
        group: String::new(),
        modes: vec![],
        default_value: None,
    }
}

/// 语义分组：生成主参数不标组名，其余组在 UI 里带小标签
fn in_group(mut p: ParamDef, group: &str) -> ParamDef {
    p.group = group.to_string();
    p
}

fn with_options(mut p: ParamDef, options: &[&str]) -> ParamDef {
    p.options = options.iter().map(|s| s.to_string()).collect();
    p
}

/// 默认值 = options 第一项（OpenAI Images 协议里各枚举的默认档）
fn first_option(mut p: ParamDef) -> ParamDef {
    if let Some(first) = p.options.first() {
        if p.default_value.is_none() {
            p.default_value = Some(ParamValue::Text(first.clone()));
        }
    }
    p
}

fn with_max(mut p: ParamDef, max: f64) -> ParamDef {
    p.min = Some(1.0);
    p.max = Some(max);
    p
}

fn advanced(mut p: ParamDef) -> ParamDef {
    p.advanced = true;
    p
}

pub fn quality_def(opts: &[&str]) -> ParamDef {
    // 默认 = options 第一项（协议默认档）
    first_option(with_options(
        param("quality", "画质", ParamKind::Select),
        opts,
    ))
}

pub fn size_def(presets: &[&str], _custom_rule: Option<SizeRule>) -> ParamDef {
    // 自定义规则存放在 ModelProfile.size_rule，控件读取同一处
    first_option(with_options(
        param("size", "尺寸", ParamKind::Size),
        presets,
    ))
}

pub fn n_def(max: f64) -> ParamDef {
    let mut p = with_max(param("n", "数量", ParamKind::Number), max);
    p.default_value = Some(ParamValue::Number(1.0));
    p
}

pub fn background_def() -> ParamDef {
    first_option(with_options(
        param("background", "背景", ParamKind::Select),
        &["auto", "transparent", "opaque"],
    ))
}

pub fn moderation_def() -> ParamDef {
    first_option(advanced(in_group(
        with_options(
            param("moderation", "审核", ParamKind::Select),
            &["auto", "low"],
        ),
        "safety",
    )))
}

pub fn output_format_def() -> ParamDef {
    first_option(in_group(
        with_options(
            param("output_format", "输出格式", ParamKind::Select),
            &["png", "jpeg", "webp"],
        ),
        "output",
    ))
}

pub fn output_compression_def() -> ParamDef {
    let mut p = advanced(in_group(
        with_max(
            param("output_compression", "压缩率", ParamKind::Number),
            100.0,
        ),
        "output",
    ));
    p.default_value = Some(ParamValue::Number(100.0));
    p
}

pub fn input_fidelity_def() -> ParamDef {
    first_option(in_group(
        with_options(
            param("input_fidelity", "原图保真", ParamKind::Select),
            &["low", "high"],
        ),
        "safety",
    ))
    .modes_edit_only()
}

trait ModesEdit {
    fn modes_edit_only(self) -> Self;
}

impl ModesEdit for ParamDef {
    fn modes_edit_only(mut self) -> Self {
        self.modes = vec![crate::model::Mode::Edit];
        self
    }
}

const SIZE_PRESETS: &[&str] = &["auto", "1024x1024", "1536x1024", "1024x1536"];

fn gpt_params(quality_opts: &[&str]) -> Vec<ParamDef> {
    vec![
        quality_def(quality_opts),
        size_def(SIZE_PRESETS, None),
        n_def(10.0),
        background_def(),
        output_format_def(),
        moderation_def(),
        output_compression_def(),
        input_fidelity_def(),
    ]
}

fn base_profile(id: &str, label: &str, params: Vec<ParamDef>) -> ModelProfile {
    ModelProfile {
        id: id.into(),
        label: label.into(),
        api: ApiKind::OpenAiImages,
        stream: true,
        max_prompt: 32_000,
        max_refs: 16,
        mask_edit: true,
        transparent: true,
        edit_note: "支持：mask + 最多 16 张参考图 + input_fidelity（high/low）".into(),
        params,
        size_rule: Some(SizeRule::gpt_image()),
        size_ratios: vec![],
    }
}

/// 尺寸比例预设：w/h 为「标准档」基准分辨率（与 OpenAI 官方预设对齐）
fn ratio(label: &str, w: u32, h: u32) -> RatioPreset {
    RatioPreset {
        label: label.into(),
        w,
        h,
    }
}

/// gpt-image-2+ 的比例预设
fn gpt_image_size_ratios() -> Vec<RatioPreset> {
    vec![
        ratio("1:1", 1024, 1024),
        ratio("3:2", 1536, 1024),
        ratio("2:3", 1024, 1536),
        ratio("16:9", 1792, 1008),
        ratio("9:16", 1008, 1792),
        ratio("4:3", 1360, 1024),
        ratio("3:4", 1024, 1360),
    ]
}

/// 按 model id 解析内置档案（provider override 可整体/部分覆盖）。
pub fn builtin(model_id: &str) -> ModelProfile {
    if model_id.starts_with("gpt-image-2.5") {
        let mut p = base_profile(
            model_id,
            "gpt-image-2.5：画质独占 xhigh / max；透明背景完整支持；任意分辨率（16 整除 · 比例 1:3–3:1 · 上限 3840×2160）",
            gpt_params(&["auto", "high", "medium", "low", "xhigh", "max"]),
        );
        p.size_ratios = gpt_image_size_ratios();
        p
    } else if model_id.starts_with("gpt-image-2") {
        let mut p = base_profile(
            model_id,
            "gpt-image-2：任意分辨率（16 整除 · 比例 1:3–3:1 · 上限 3840×2160）；透明背景为预览特性",
            gpt_params(&["auto", "high", "medium", "low"]),
        );
        p.size_ratios = gpt_image_size_ratios();
        p
    } else if model_id.starts_with("gpt-image") {
        let mut p = base_profile(
            model_id,
            "gpt-image-1 系：固定三档尺寸 + auto；quality high/medium/low；恒返 b64_json",
            gpt_params(&["auto", "high", "medium", "low"]),
        );
        p.size_rule = None;
        p
    } else {
        // 未识别模型：按通用 OpenAI 兼容协议放行常用参数，实际以网关为准
        let mut p = base_profile(
            model_id,
            "未识别模型：参数以 Provider / 网关为准，发送前请自行确认",
            vec![
                quality_def(&["auto", "high", "medium", "low"]),
                size_def(&["auto", "1024x1024", "1536x1024", "1024x1536"], None),
                n_def(10.0),
                background_def(),
                output_format_def(),
            ],
        );
        p.api = ApiKind::Generic;
        p.stream = false;
        p.size_rule = None;
        p
    }
}

/// 合并 provider 的元数据覆盖（深合并：override 的字段优先）。
pub fn merged(model_id: &str, override_json: Option<&serde_json::Value>) -> ModelProfile {
    let base = builtin(model_id);
    match override_json {
        None | Some(serde_json::Value::Null) => base,
        Some(ov) => {
            let mut base_json = serde_json::to_value(&base).expect("profile serialize");
            deep_merge(&mut base_json, ov);
            match serde_json::from_value(base_json) {
                Ok(p) => p,
                Err(_) => base, // 覆盖损坏时回退内置档案
            }
        }
    }
}

fn deep_merge(base: &mut serde_json::Value, ov: &serde_json::Value) {
    match (base, ov) {
        (serde_json::Value::Object(b), serde_json::Value::Object(o)) => {
            for (k, v) in o {
                if v.is_null() {
                    b.remove(k);
                } else {
                    deep_merge(b.entry(k.clone()).or_insert(serde_json::Value::Null), v);
                }
            }
        }
        (b, o) => *b = o.clone(),
    }
}

/// 能力徽章：从档案推导（所见即所配，override 后自动跟随）
pub fn badges(p: &ModelProfile) -> Vec<(&'static str, &'static str)> {
    let mut out = vec![];
    if p.size_rule.is_some() && p.find_param("size").is_some() {
        out.push(("任意尺寸", "ok"));
    }
    if p.find_param("background").is_some() {
        out.push(("透明底", "ok"));
    }
    if p.stream {
        out.push(("流式", "ok"));
    }
    if let Some(q) = p.find_param("quality") {
        if q.options.iter().any(|o| o == "xhigh" || o == "max") {
            out.push(("xhigh/max", "info"));
        }
    }
    if p.mask_edit {
        out.push(("mask 编辑", "ok"));
    }
    if p.api == ApiKind::Generic {
        out.push(("网关模型", "warn"));
    }
    out
}
