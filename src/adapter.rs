//! 协议适配层（server 独占）。
//!
//! 接收「渲染后的执行请求」（完整 prompt + 参数 + 图片列表），
//! 转换为具体 API 调用。Phase 1 实现 OpenAI Images 兼容协议；
//! 未来 seeddream / nano banana 等各加一个分支，UI 无感知。

use crate::model::{AssetRef, ModelProfile, ParamMap, ParamValue, Provider, RunStatus, Usage};
use crate::store;

/// 渲染后的执行请求：prompt 已填好变量，图片已按发送顺序排列
pub struct ExecRequest<'a> {
    pub prompt: &'a str,
    pub params: &'a ParamMap,
    /// 固定参考图在前、槽位图在后，顺序即 image[] 顺序
    pub images: &'a [AssetRef],
    pub mask: Option<&'a AssetRef>,
    pub model_id: &'a str,
    pub profile: &'a ModelProfile,
}

/// 把统一参数映射为协议请求体字段（Unset 跳过；未知键按 api 种类处理）
fn params_to_body(profile: &ModelProfile, params: &ParamMap) -> serde_json::Value {
    let mut body = serde_json::Map::new();
    for (key, value) in params {
        if value.is_unset() {
            continue;
        }
        let api_key = match profile.find_param(key) {
            Some(def) => def.api_key().to_string(),
            // 无档案定义的键：仅通用协议透传，避免把不认识的字段发给正式 API
            None => {
                if profile.api == crate::model::ApiKind::Generic {
                    key.clone()
                } else {
                    continue;
                }
            }
        };
        if let Some(v) = value.to_request_json() {
            body.insert(api_key, v);
        }
    }
    serde_json::Value::Object(body)
}

fn normalize_base(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

/// API Key 三种填法：
/// 1. `sk-…` 等字面密钥，原样使用
/// 2. `env:变量名` 显式引用
/// 3. 纯大写+下划线的值（如 OPENAI_API_KEY）自动当作环境变量名读取
/// 均在请求时从服务端进程的环境变量解析，密钥本身不落在配置文件里。
fn resolve_api_key(stored: &str) -> Result<String, (RunStatus, String)> {
    let stored = stored.trim();
    if let Some(var) = stored.strip_prefix("env:") {
        let var = var.trim();
        if var.is_empty() {
            return Err((RunStatus::Error, "env: 后缺少变量名".into()));
        }
        read_env_var(var)
    } else if is_env_name(stored) {
        // 纯大写+下划线（至少含一个字母）即视为环境变量名风格
        read_env_var(stored)
    } else {
        Ok(stored.to_string())
    }
}

/// 纯大写+下划线（至少含一个字母）即视为环境变量名风格
fn is_env_name(s: &str) -> bool {
    !s.is_empty()
        && s.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        && s.chars().any(|c| c.is_ascii_alphabetic())
}

fn read_env_var(var: &str) -> Result<String, (RunStatus, String)> {
    match std::env::var(var) {
        Ok(v) if !v.trim().is_empty() => Ok(v.trim().to_string()),
        Ok(_) => Err((RunStatus::Error, format!("环境变量 {var} 的值为空"))),
        Err(_) => Err((RunStatus::Error, format!(
            "环境变量 {var} 未设置（后端进程看不到它；若想直接填密钥，请用 sk-… 这样的字面值）"
        ))),
    }
}

pub struct GenOutcome {
    pub images: Vec<AssetRef>,
    pub usage: Option<Usage>,
}

/// 执行一次生成。图片非空走 edits（multipart），否则走 generations（JSON）。
pub async fn execute(
    provider: &Provider,
    req: ExecRequest<'_>,
) -> Result<GenOutcome, (RunStatus, String)> {
    let base = normalize_base(&provider.base_url);
    if base.is_empty() {
        return Err((RunStatus::Error, "Provider 未配置 Base URL".into()));
    }

    let api_key = resolve_api_key(&provider.api_key)?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| (RunStatus::Error, format!("HTTP client 初始化失败：{e}")))?;

    let mut request = client
        .post(format!("{base}/images/generations"))
        .header("Authorization", format!("Bearer {}", api_key));

    if req.images.is_empty() {
        // ---- 文生图：JSON ----
        let mut body = serde_json::Map::new();
        body.insert("model".into(), req.model_id.into());
        body.insert("prompt".into(), req.prompt.into());
        if let serde_json::Value::Object(extra) = params_to_body(req.profile, req.params) {
            for (k, v) in extra {
                body.insert(k, v);
            }
        }
        request = request.json(&serde_json::Value::Object(body));
    } else {
        // ---- 图片编辑：multipart，固定参考图在前、槽位图在后 ----
        let mut form = reqwest::multipart::Form::new()
            .text("model", req.model_id.to_string())
            .text("prompt", req.prompt.to_string());

        let multi = req.images.len() > 1;
        for (i, r#ref) in req.images.iter().enumerate() {
            let bytes = store::read_asset(r#ref)
                .await
                .map_err(|e| (RunStatus::Error, e))?;
            let part = reqwest::multipart::Part::bytes(bytes)
                .file_name(format!("image-{i}.{}", r#ref.ext));
            form = if multi {
                form.part("image[]", part)
            } else {
                form.part("image", part)
            };
        }

        if let Some(mask) = &req.mask {
            let bytes = store::read_asset(mask)
                .await
                .map_err(|e| (RunStatus::Error, e))?;
            form = form.part(
                "mask",
                reqwest::multipart::Part::bytes(bytes).file_name("mask.png"),
            );
        }

        for (key, value) in req.params {
            if value.is_unset() {
                continue;
            }
            let Some(def) = req.profile.find_param(key) else { continue };
            let api_key = def.api_key().to_string();
            let text = match value {
                ParamValue::Number(n) => {
                    if n.fract() == 0.0 {
                        format!("{}", *n as i64)
                    } else {
                        format!("{n}")
                    }
                }
                ParamValue::Text(s) | ParamValue::Size(s) => s.clone(),
                ParamValue::Unset => continue,
            };
            form = form.text(api_key, text);
        }

        request = request.multipart(form);
    }

    let resp = request
        .send()
        .await
        .map_err(|e| (RunStatus::Error, format!("请求失败：{e}")))?;

    let status = resp.status();
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| (RunStatus::Error, format!("读取响应失败：{e}")))?;

    let json: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| {
        let text = String::from_utf8_lossy(&bytes);
        (
            RunStatus::Error,
            format!("HTTP {status} · 响应不是 JSON：{}", &text[..text.len().min(300)]),
        )
    })?;

    if !status.is_success() {
        let msg = json
            .pointer("/error/message")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err((RunStatus::Error, msg));
    }

    // 解析 data[]：b64_json 或 url
    let mut images = vec![];
    if let Some(items) = json.get("data").and_then(|v| v.as_array()) {
        for item in items {
            if let Some(b64) = item.get("b64_json").and_then(|v| v.as_str()) {
                use base64::Engine;
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(b64)
                    .map_err(|e| (RunStatus::Error, format!("b64 解码失败：{e}")))?;
                let ext = sniff_ext(&bytes);
                let asset = store::save_asset(&bytes, ext)
                    .await
                    .map_err(|e| (RunStatus::Error, e))?;
                images.push(asset);
            } else if let Some(url) = item.get("url").and_then(|v| v.as_str()) {
                let img = client
                    .get(url)
                    .send()
                    .await
                    .map_err(|e| (RunStatus::Error, format!("拉取结果图失败：{e}")))?;
                let bytes = img
                    .bytes()
                    .await
                    .map_err(|e| (RunStatus::Error, format!("拉取结果图失败：{e}")))?;
                let ext = sniff_ext(&bytes);
                let asset = store::save_asset(&bytes, ext)
                    .await
                    .map_err(|e| (RunStatus::Error, e))?;
                images.push(asset);
            }
        }
    }

    if images.is_empty() {
        return Err((RunStatus::Error, "响应中没有图片数据".into()));
    }

    let usage = json.get("usage").map(|u| Usage {
        input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()),
        output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()),
        total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()),
        image_tokens: u
            .pointer("/input_tokens_details/image_tokens")
            .and_then(|v| v.as_u64()),
    });

    Ok(GenOutcome { images, usage })
}

fn sniff_ext(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        "png"
    } else if bytes.starts_with(b"\xff\xd8\xff") {
        "jpg"
    } else if bytes.len() > 12 && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else {
        "png"
    }
}
