//! 统一数据模型。
//!
//! 核心思想：
//! - 模型能力以 `ModelProfile`（含 params schema）元数据描述，请求使用统一的
//!   `ParamKey → ParamValue` 表示；
//! - Graph 保存节点结构与参数，GraphView 只保存布局和视口。
//! - 运行状态与产物由前端节点在会话内持有，不建立运行档案。
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 统一参数值。Unset = 该参数不随请求发送。
/// 注意必须用相邻标签（tag+content）：serde 的内部标签不支持含 String/数字的新类型变体。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "t", content = "v", rename_all = "snake_case")]
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
    /// 参数的协议默认值。UI 初值与「始终完整发送」的归一化都以它为准：
    /// 缺失该键 = 按此值显式发送（None = 不强制，如 text 类可空参数）
    #[serde(default)]
    pub default_value: Option<ParamValue>,
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
            note: "边长需被 16 整除 · 宽高比 1:3–3:1 · 上限 3840×2160（超 2560×1440 为实验性）"
                .into(),
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
            s = s.min(rule.max_w as f64 / fw).min(rule.max_h as f64 / fh);
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

/// 图目录（可嵌套；parent_id = None 为根）。删除目录时其图回到未分组。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GraphGroup {
    pub id: String,
    pub name: String,
    /// 父目录；None = 根级
    #[serde(default)]
    pub parent_id: Option<String>,
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

// ---------- 节点图文档 ----------

/// 节点种类（稳定枚举；前端 nodeFactories 按同名字符串对应）
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Model,
    Prompt,
    Image,
    Generate,
    Preview,
}

/// 节点参数：统一键 → 标量（字符串 / 数字）。结构与 UI 控件解耦。
pub type NodeParams = BTreeMap<String, serde_json::Value>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GraphNode {
    pub id: String,
    pub r#type: NodeType,
    #[serde(default)]
    pub params: NodeParams,
}

/// 端口级连线。id 为派生式（source:port->target:port），天然幂等。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GraphEdge {
    pub id: String,
    pub source: String,
    #[serde(rename = "sourcePort")]
    pub source_port: String,
    pub target: String,
    #[serde(rename = "targetPort")]
    pub target_port: String,
}

/// 结构文档：UI 无关的事实源。不含坐标/视口等表现信息（见 GraphView）。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Graph {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub nodes: Vec<GraphNode>,
    #[serde(default)]
    pub edges: Vec<GraphEdge>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// 表现文档：布局 / 视口。可丢可重建，保存不推进 updated_at。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct GraphView {
    #[serde(default)]
    pub positions: BTreeMap<String, Position>,
    #[serde(default)]
    pub viewport: Option<Viewport>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Viewport {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

// ---------- 请求与校验 ----------

/// 请求级校验：模型已选、图片总数上限、参数取值（按 profile 元数据）。
/// 把缺失的协议参数补齐为档案默认值（「始终完整发送」）。
/// UI 侧同样会填默认，这里是兜底：老图、直接调 /api/providers/{provider_id}/generate 的路径
/// 都保证发出的请求参数完整。text 类（default_value=None）不补。
pub fn with_defaults(profile: &ModelProfile, params: &ParamMap) -> ParamMap {
    let mut out = params.clone();
    for def in &profile.params {
        if out.contains_key(&def.key) {
            continue;
        }
        if let Some(dv) = &def.default_value {
            out.insert(def.key.clone(), dv.clone());
        }
    }
    out
}

pub fn validate_request(
    profile: &ModelProfile,
    model_id: &str,
    params: &ParamMap,
    image_count: usize,
) -> Result<(), String> {
    if model_id.is_empty() {
        return Err("请先选择模型".into());
    }
    if image_count > profile.max_refs as usize {
        return Err(format!("图片总数超过上限 {} 张", profile.max_refs));
    }
    for (key, value) in params {
        let Some(def) = profile.find_param(key) else {
            continue;
        };
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

// ---------- 本次生成用量 ----------

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_defaults_fills_all_protocol_params() {
        let profile = crate::profiles::merged("gpt-image-2", None);
        // 空参数 → 全部补齐为档案默认
        let full = with_defaults(&profile, &ParamMap::new());
        for def in &profile.params {
            if def.default_value.is_some() {
                assert!(full.contains_key(&def.key), "默认值未覆盖 {}", def.key);
            }
        }
        // 具体默认值抽查
        assert_eq!(full.get("quality"), Some(&ParamValue::Text("auto".into())));
        assert_eq!(full.get("size"), Some(&ParamValue::Text("auto".into())));
        assert_eq!(
            full.get("background"),
            Some(&ParamValue::Text("auto".into()))
        );
        assert_eq!(
            full.get("output_format"),
            Some(&ParamValue::Text("png".into()))
        );
        assert_eq!(
            full.get("input_fidelity"),
            Some(&ParamValue::Text("low".into()))
        );
        assert_eq!(full.get("n"), Some(&ParamValue::Number(1.0)));
        assert_eq!(
            full.get("output_compression"),
            Some(&ParamValue::Number(100.0))
        );
        // user 已从面板与档案移除：不发送、也无默认
        assert!(!profile.params.iter().any(|p| p.key == "user"));
        assert!(!full.contains_key("user"));
        // 显式值不被默认覆盖
        let with_size = with_defaults(&profile, &full);
        assert_eq!(with_size.get("quality"), full.get("quality"));
        assert_eq!(
            with_size.get("size"),
            Some(&ParamValue::Text("auto".into()))
        );
    }

    #[test]
    fn graph_serde_roundtrip() {
        let g = Graph {
            id: "g1".into(),
            title: "测试图".into(),
            group_id: None,
            nodes: vec![
                GraphNode {
                    id: "n1".into(),
                    r#type: NodeType::Model,
                    params: serde_json::json!({
                        "provider": "openai",
                        "modelId": "gpt-image-2"
                    })
                    .as_object()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .collect(),
                },
                GraphNode {
                    id: "n2".into(),
                    r#type: NodeType::Preview,
                    params: Default::default(),
                },
            ],
            edges: vec![GraphEdge {
                id: "n1:model->n2:image".into(),
                source: "n1".into(),
                source_port: "model".into(),
                target: "n2".into(),
                target_port: "image".into(),
            }],
            created_at: 1,
            updated_at: 2,
        };
        let bytes = serde_json::to_vec(&g).unwrap();
        let back: Graph = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, g);
        // camelCase 端口字段 + snake_case 类型标签可读
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"sourcePort\":\"model\""));
        assert!(text.contains("\"type\":\"model\""));
    }

    #[test]
    fn graph_default_fields_tolerate_old_json() {
        let g: Graph =
            serde_json::from_str(r#"{"id":"g1","created_at":1,"updated_at":2}"#).unwrap();
        assert!(g.nodes.is_empty());
    }

    #[test]
    fn graph_view_roundtrip() {
        let mut v = GraphView::default();
        v.positions
            .insert("n1".into(), Position { x: 10.0, y: 20.0 });
        v.viewport = Some(Viewport {
            x: -5.0,
            y: 0.0,
            zoom: 1.2,
        });
        let s = serde_json::to_string(&v).unwrap();
        let back: GraphView = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
        // 空对象也可读（首次保存前 GET）
        let empty: GraphView = serde_json::from_str("{}").unwrap();
        assert!(empty.positions.is_empty());
    }

    #[test]
    fn param_value_serde_roundtrip() {
        // 带值变体必须可序列化（此前内部标签下会直接 Err 导致静默丢写）
        for v in [
            ParamValue::Text("hello".into()),
            ParamValue::Number(2.0),
            ParamValue::Size("1024x1024".into()),
            ParamValue::Unset,
        ] {
            let s = serde_json::to_string(&v).unwrap();
            assert_eq!(serde_json::from_str::<ParamValue>(&s).unwrap(), v);
        }
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
