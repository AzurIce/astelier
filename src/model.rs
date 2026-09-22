//! 统一数据模型 —— 客户端 / 服务端共享。
//!
//! 核心思想：模型能力以 `ModelProfile`（含 params schema）元数据描述，
//! UI 表单与输入持久化都使用统一的 `ParamKey → ParamValue` 表示；
//! 发送请求时由 adapter 按 `api` 种类转换成各协议的具体字段。

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 统一参数值。Unset = 该参数不随请求发送。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ParamValue {
    Unset,
    Text(String),
    Number(f64),
    /// 尺寸：预设档（如 "1024x1024"、"auto"）或自定义 "WxH"
    Size(String),
}

// 客户端仅持久化/展示该值；转换方法只在服务端 adapter 路径使用
#[allow(dead_code)]
impl ParamValue {
    pub fn is_unset(&self) -> bool {
        matches!(self, ParamValue::Unset)
    }

    /// 转成请求体里的 JSON 值（Unset → None，由调用方跳过）
    pub fn to_request_json(&self) -> Option<serde_json::Value> {
        match self {
            ParamValue::Unset => None,
            ParamValue::Text(s) => Some(serde_json::Value::String(s.clone())),
            ParamValue::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 9e15 {
                    Some(serde_json::Value::Number((*n as i64).into()))
                } else {
                    serde_json::Number::from_f64(*n).map(serde_json::Value::Number)
                }
            }
            ParamValue::Size(s) => Some(serde_json::Value::String(s.clone())),
        }
    }
}

pub type ParamMap = BTreeMap<String, ParamValue>;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Gen,
    Edit,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Gen => "文生图",
            Mode::Edit => "编辑",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ParamKind {
    Select,
    Number,
    Text,
    Size,
}

/// 参数描述符：统一键 + API 字段名 + 控件类型。
/// 接入新模型 = 配一份 params 元数据，不写代码。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ParamDef {
    /// 统一键（如 "quality"），ParamMap 与 UI 用它寻址
    pub key: String,
    /// 发送到请求体的字段名；空串 = 同 key
    #[serde(default)]
    pub api_key: String,
    pub label: String,
    pub kind: ParamKind,
    #[serde(default)]
    pub options: Vec<String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    /// 归入「更多参数」折叠区
    #[serde(default)]
    pub advanced: bool,
    /// 语义分组："" = 生成主参数，"output" = 输出，"safety" = 审核与标识
    #[serde(default)]
    pub group: String,
    /// 限定出现的模式；空 = gen + edit 都出现
    #[serde(default)]
    pub modes: Vec<Mode>,
}

impl ParamDef {
    pub fn api_key(&self) -> &str {
        if self.api_key.is_empty() {
            &self.key
        } else {
            &self.api_key
        }
    }

    pub fn visible_in(&self, mode: Mode) -> bool {
        self.modes.is_empty() || self.modes.contains(&mode)
    }
}

/// 自定义尺寸约束规则（gpt-image-2+：边长被 16 整除、比例 1:3–3:1、上限 3840×2160）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SizeRule {
    pub step: u32,
    pub min_side: u32,
    pub max_w: u32,
    pub max_h: u32,
    pub ratio_min: f64,
    pub ratio_max: f64,
    pub note: String,
}

impl SizeRule {
    pub fn gpt_image() -> Self {
        Self {
            step: 16,
            min_side: 16,
            max_w: 3840,
            max_h: 2160,
            ratio_min: 1.0 / 3.0,
            ratio_max: 3.0,
            note: "边长需被 16 整除 · 宽高比 1:3–3:1 · 上限 3840×2160（超 2560×1440 为实验性）".into(),
        }
    }

    pub fn validate(&self, w: i64, h: i64) -> Result<(), String> {
        if w < self.min_side as i64 || h < self.min_side as i64 {
            return Err(format!("宽高都不能小于 {}", self.min_side));
        }
        if w % self.step as i64 != 0 || h % self.step as i64 != 0 {
            return Err(format!("宽和高都必须能被 {} 整除", self.step));
        }
        let ratio = w as f64 / h as f64;
        if !(self.ratio_min..=self.ratio_max).contains(&ratio) {
            return Err("宽高比必须在 1:3 到 3:1 之间".into());
        }
        if w > self.max_w as i64 || h > self.max_h as i64 {
            return Err(format!("分辨率上限 {}x{}", self.max_w, self.max_h));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKind {
    /// OpenAI Images 兼容协议（/images/generations、/images/edits）
    OpenAiImages,
    /// 未识别协议：JSON 直发 generations，字段名照搬 api_key，以网关为准
    Generic,
}

/// 尺寸控件的比例预设：w/h 表达宽高比（也作为该比例 1.0MP 时的参考尺寸）
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RatioPreset {
    pub label: String,
    pub w: u32,
    pub h: u32,
}

/// 模型能力档案：UI 参数区、校验、请求转换全部由它驱动。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ModelProfile {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub api: ApiKind,
    pub stream: bool,
    pub max_prompt: u32,
    pub max_refs: u32,
    pub mask_edit: bool,
    pub transparent: bool,
    pub edit_note: String,
    #[serde(default)]
    pub params: Vec<ParamDef>,
    #[serde(default)]
    pub size_rule: Option<SizeRule>,
    /// 比例预设（空 = 退回扁平 presets 列表）；分辨率由「比例 × 目标百万像素」计算
    #[serde(default)]
    pub size_ratios: Vec<RatioPreset>,
}

impl ModelProfile {
    pub fn params_for(&self, mode: Mode, advanced: bool) -> Vec<&ParamDef> {
        self.params
            .iter()
            .filter(|d| d.visible_in(mode))
            .filter(|d| if advanced { d.advanced } else { !d.advanced })
            .collect()
    }

    pub fn find_param(&self, key: &str) -> Option<&ParamDef> {
        self.params.iter().find(|d| d.key == key)
    }

    /// 比例 + 目标总百万像素 → 具体 WxH。
    /// 1.0 MP ≈ 1024×1024；总像素按比例分配到宽高，受 SizeRule
    /// 上限约束时等比缩小，边长对齐 step。
    pub fn compute_mp_size(&self, ratio_w: u32, ratio_h: u32, mp: f64) -> (u32, u32) {
        let (step, max_w, max_h, min_side) = match &self.size_rule {
            Some(rule) => (rule.step, rule.max_w, rule.max_h, rule.min_side),
            None => (16, 8192, 8192, 16),
        };
        let pixels = (mp.max(0.05)) * 1024.0 * 1024.0;
        let r = ratio_w as f64 / ratio_h as f64;
        let fw = (pixels * r).sqrt();
        let fh = (pixels / r).sqrt();
        let mut s = 1.0f64;
        if let Some(rule) = &self.size_rule {
            s = s
                .min(rule.max_w as f64 / fw)
                .min(rule.max_h as f64 / fh);
        }
        let snap = |v: f64| -> u32 {
            let n = ((v / step as f64).round() as u64).max(1);
            (n as u32) * step
        };
        (
            snap(fw * s).clamp(min_side, max_w),
            snap(fh * s).clamp(min_side, max_h),
        )
    }

    /// 宽高互换后夹回规则上限（等比缩放 + 边长对齐 step）
    #[allow(dead_code)]
    pub fn clamp_size(&self, w: u32, h: u32) -> (u32, u32) {
        let (step, max_w, max_h, min_side) = match &self.size_rule {
            Some(rule) => (rule.step, rule.max_w, rule.max_h, rule.min_side),
            None => (16, 8192, 8192, 16),
        };
        let mut s = 1.0f32;
        if let Some(rule) = &self.size_rule {
            s = s
                .min(rule.max_w as f32 / w as f32)
                .min(rule.max_h as f32 / h as f32);
        }
        let snap = |v: f32| -> u32 {
            let n = ((v / step as f32).round() as u32).max(1);
            n * step
        };
        (
            snap(w as f32 * s).clamp(min_side, max_w),
            snap(h as f32 * s).clamp(min_side, max_h),
        )
    }

}

/// 资产引用（生成的图 / 上传的参考图 / mask），文件存服务端 data/assets/
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AssetRef {
    pub id: String,
    pub ext: String,
    #[serde(default)]
    pub w: Option<u32>,
    #[serde(default)]
    pub h: Option<u32>,
}

impl AssetRef {
    pub fn url(&self) -> String {
        format!("/asset/{}.{}", self.id, self.ext)
    }
}

/// 输入分组（文件夹）。删除分组时其输入回到未分组，不连带删除。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InputGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub created_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Provider {
    pub id: String,
    pub name: String,
    /// 含 /v1 的根地址，如 https://api.openai.com/v1
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub models: Vec<String>,
    /// 模型元数据覆盖（provider × model），JSON 结构同 ModelProfile 的子集
    #[serde(default)]
    pub overrides: BTreeMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Config {
    pub providers: Vec<Provider>,
    /// 当前激活的 provider id
    pub active_provider: String,
}

impl Config {
    pub fn active(&self) -> Option<&Provider> {
        self.providers.iter().find(|p| p.id == self.active_provider)
    }
}

/// 配方：可复用的创作定义 —— prompt 模板 + 参数 + 固定参考图。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Recipe {
    pub id: String,
    pub provider_id: String,
    pub model_id: String,
    #[serde(default)]
    pub prompt_template: String,
    /// 手动标题；空 = 从模板推导
    #[serde(default)]
    pub title: Option<String>,
    /// 所属分组；None = 未分组
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub params: ParamMap,
    /// 固定参考图（编辑区维护，随配方走）
    #[serde(default)]
    pub refs: Vec<AssetRef>,
    #[serde(default)]
    pub mask: Option<AssetRef>,
    /// 每次修改模板/参数/固定图后 +1；Run 记录当时的版本号
    #[serde(default)]
    pub version: u32,
    pub created_at: u64,
    pub updated_at: u64,
}

impl Recipe {
    #[allow(dead_code)]
    pub fn mode(&self) -> Mode {
        if self.refs.is_empty() {
            Mode::Gen
        } else {
            Mode::Edit
        }
    }

    pub fn display_title(&self) -> String {
        if let Some(t) = self.title.as_deref() {
            let t = t.trim();
            if !t.is_empty() {
                return truncate_label(t);
            }
        }
        let line = self
            .prompt_template
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty() && !l.contains('{'))
            .unwrap_or("");
        if line.is_empty() {
            "未命名配方".into()
        } else {
            truncate_label(line)
        }
    }

    /// 参数校验（模板与槽位由 render 校验）
    pub fn validate_params(&self, profile: &ModelProfile) -> Result<(), String> {
        if self.model_id.is_empty() {
            return Err("请先选择模型".into());
        }
        if self.refs.len() > profile.max_refs as usize {
            return Err(format!("固定参考图超过上限 {} 张", profile.max_refs));
        }
        for (key, value) in &self.params {
            let Some(def) = profile.find_param(key) else { continue };
            if let (Some(max), ParamValue::Number(n)) = (def.max, value) {
                if *n > max || *n < def.min.unwrap_or(f64::NEG_INFINITY) {
                    return Err(format!("「{}」超出范围", def.label));
                }
            }
            if def.kind == ParamKind::Select {
                if let ParamValue::Text(s) = value {
                    if !def.options.contains(s) {
                        return Err(format!("「{}」的取值 {s} 不在可选列表", def.label));
                    }
                }
            }
        }
        Ok(())
    }
}

/// 配方下的输入：一份变量值 + 槽位图片，是模板的一次具体化。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecipeInput {
    pub id: String,
    pub recipe_id: String,
    /// 手动标题；空 = 自动推导
    #[serde(default)]
    pub title: Option<String>,
    /// 文字槽 {xxx} 的值
    #[serde(default)]
    pub variables: BTreeMap<String, String>,
    /// 图片槽 {img:xxx} → 图片；未出现的槽 = 未绑定
    #[serde(default)]
    pub images: BTreeMap<String, AssetRef>,
    pub created_at: u64,
    pub updated_at: u64,
}

impl RecipeInput {
    pub fn display_title(&self, index: usize) -> String {
        if let Some(t) = self.title.as_deref() {
            let t = t.trim();
            if !t.is_empty() {
                return truncate_label(t);
            }
        }
        if let Some(v) = self.variables.values().next() {
            let v = v.trim();
            if !v.is_empty() {
                return truncate_label(v);
            }
        }
        format!("输入 {}", index + 1)
    }
}

// ---------- Prompt 模板 ----------

/// 模板片段：{xxx} 文字槽、{img:xxx} 图片槽、其余为字面文本
#[derive(Clone, Debug, PartialEq)]
pub enum TemplatePiece {
    Text(String),
    Var(String),
    Img(String),
}

/// 解析模板。`{{` 为 `{` 的转义；未闭合的 `{` 按字面处理。
pub fn parse_template(template: &str) -> Vec<TemplatePiece> {
    let mut out = vec![];
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            if chars.peek() == Some(&'{') {
                chars.next();
                text.push('{');
                continue;
            }
            let mut inner = String::new();
            let mut closed = false;
            for c2 in chars.by_ref() {
                if c2 == '}' {
                    closed = true;
                    break;
                }
                inner.push(c2);
            }
            if !closed {
                text.push('{');
                text.push_str(&inner);
                continue;
            }
            let name = inner.trim();
            if let Some(img) = name.strip_prefix("img:") {
                let img = img.trim();
                if !img.is_empty() {
                    if !text.is_empty() {
                        out.push(TemplatePiece::Text(std::mem::take(&mut text)));
                    }
                    out.push(TemplatePiece::Img(img.into()));
                    continue;
                }
            }
            if !name.is_empty() {
                if !text.is_empty() {
                    out.push(TemplatePiece::Text(std::mem::take(&mut text)));
                }
                out.push(TemplatePiece::Var(name.into()));
                continue;
            }
            // 空槽按字面输出
            text.push('{');
            text.push_str(&inner);
            text.push('}');
        } else {
            text.push(c);
        }
    }
    if !text.is_empty() {
        out.push(TemplatePiece::Text(text));
    }
    out
}

/// 模板里的全部槽名（去重，保持首次出现顺序）
pub fn template_variables(template: &str) -> (Vec<String>, Vec<String>) {
    let mut vars: Vec<String> = vec![];
    let mut imgs: Vec<String> = vec![];
    for piece in parse_template(template) {
        match piece {
            TemplatePiece::Var(v) => {
                if !vars.contains(&v) {
                    vars.push(v);
                }
            }
            TemplatePiece::Img(i) => {
                if !imgs.contains(&i) {
                    imgs.push(i);
                }
            }
            TemplatePiece::Text(_) => {}
        }
    }
    (vars, imgs)
}

/// 渲染结果
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RenderedPrompt {
    pub prompt: String,
    /// 按模板出现顺序排列的槽位图片（拼在固定参考图之后发送）
    pub slot_images: Vec<AssetRef>,
    pub missing_vars: Vec<String>,
    pub missing_imgs: Vec<String>,
}

impl RenderedPrompt {
    pub fn is_complete(&self) -> bool {
        self.missing_vars.is_empty() && self.missing_imgs.is_empty()
    }
}

/// 用输入的变量值与槽位图片渲染模板。
/// `base_refs` 为配方固定参考图数量——{img:xxx} 渲染为 [图N]，
/// N 是它在「固定图 + 槽位图」合并序列中的位置（从 1 起）。
pub fn render_recipe(
    template: &str,
    values: &BTreeMap<String, String>,
    images: &BTreeMap<String, AssetRef>,
    base_refs: usize,
) -> RenderedPrompt {
    let mut out = RenderedPrompt::default();
    let mut slot_seen: Vec<String> = vec![];
    for piece in parse_template(template) {
        match piece {
            TemplatePiece::Text(t) => out.prompt.push_str(&t),
            TemplatePiece::Var(name) => match values.get(&name).map(|s| s.trim()) {
                Some(v) if !v.is_empty() => out.prompt.push_str(v),
                _ => {
                    out.missing_vars.push(name.clone());
                    out.prompt.push('{');
                    out.prompt.push_str(&name);
                    out.prompt.push('}');
                }
            },
            TemplatePiece::Img(name) => {
                let idx = if let Some(pos) = slot_seen.iter().position(|s| s == &name) {
                    pos
                } else {
                    slot_seen.push(name.clone());
                    slot_seen.len() - 1
                };
                out.prompt.push_str(&format!("[图{}]", base_refs + idx + 1));
                if let Some(asset) = images.get(&name) {
                    if out.slot_images.len() == idx {
                        out.slot_images.push(asset.clone());
                    } else {
                        out.slot_images[idx] = asset.clone();
                    }
                } else {
                    out.missing_imgs.push(name.clone());
                }
            }
        }
    }
    out
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RunStatus {
    Running,
    Done,
    Error,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Usage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub image_tokens: Option<u64>,
}

/// 一次生成 = 一个批次。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Run {
    pub id: String,
    /// 出自哪个配方
    pub recipe_id: String,
    /// 出自哪个输入（快捷运行也会自动落成输入）
    pub input_id: Option<String>,
    /// 执行时配方的版本号
    pub recipe_version: u32,
    pub provider_id: String,
    pub model_id: String,
    pub mode: Mode,
    /// 渲染后的完整 prompt（快照）
    pub prompt: String,
    /// 参数快照
    #[serde(default)]
    pub params: ParamMap,
    /// 实际发送的图片总数（固定参考图 + 槽位图）
    #[serde(default)]
    pub ref_count: usize,
    pub status: RunStatus,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub images: Vec<AssetRef>,
    #[serde(default)]
    pub usage: Option<Usage>,
    pub created_at: u64,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

fn truncate_label(s: &str) -> String {
    if s.chars().count() > 42 {
        let head: String = s.chars().take(42).collect();
        format!("{head}…")
    } else {
        s.to_string()
    }
}

/// 客户端 / 服务端共享的轻量工具
pub mod fmt {
    use super::*;

    pub fn relative_time(ts_ms: u64, now_ms: u64) -> String {
        let diff = now_ms.saturating_sub(ts_ms);
        let sec = diff / 1000;
        match sec {
            0..=44 => "刚刚".into(),
            45..=89 => "1 分钟前".into(),
            90..=2699 => format!("{} 分钟前", sec / 60),
            2700..=5339 => "1 小时前".into(),
            5340..=86_069 => format!("{} 小时前", sec / 3600),
            86_070..=172_739 => "1 天前".into(),
            _ => format!("{} 天前", sec / 86_400),
        }
    }

    pub fn usage_text(u: &Usage) -> String {
        let mut parts = vec![];
        if let Some(t) = u.total_tokens {
            parts.push(format!("tokens {t}"));
        }
        if let Some(i) = u.image_tokens {
            parts.push(format!("图入 {i}"));
        }
        if let Some(o) = u.output_tokens {
            parts.push(format!("出 {o}"));
        }
        parts.join(" · ")
    }
}
