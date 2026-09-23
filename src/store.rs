//! 服务端持久化：data/ 下的目录结构 + 资产文件。
//!
//! ```text
//! data/
//! ├── config.json            Provider 配置
//! ├── groups.json            节点图分组（文件夹）
//! ├── assets/{id}.{ext}      全部图片资产（统一池，被各方按 id 引用）
//! ├── graphs/{gid}/
//! │   └── graph.json         节点图：节点（种类/位置/装配内容）+ 连线
//! └── runs/{run_id}.json     批次档案：状态 + 最终请求归档（旧批次为配方快照）
//! ```
//!
//! 单用户本地工具，JSON 落盘足够；写入用 tmp+rename 原子替换。

use crate::model::{AssetRef, Config, Graph, GraphGroup, Run};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;

const DATA_DIR: &str = "data";

static STORE_LOCK: Mutex<()> = Mutex::const_new(());

fn dir() -> PathBuf {
    Path::new(DATA_DIR).to_path_buf()
}

fn sub_dir(parts: &[&str]) -> PathBuf {
    let mut p = dir();
    for part in parts {
        p.push(part);
    }
    p
}

async fn read_json_at<T: DeserializeOwned>(path: &Path) -> Option<T> {
    match tokio::fs::read(path).await {
        Ok(bytes) => serde_json::from_slice(&bytes).ok(),
        Err(_) => None,
    }
}

async fn read_json<T: DeserializeOwned>(rel: &[&str]) -> Option<T> {
    read_json_at(&sub_dir(rel)).await
}

/// 无锁内部写。仅供已持有 STORE_LOCK 的调用方使用。
async fn write_json_at_unlocked<T: serde::Serialize>(path: &Path, value: &T) {
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    let tmp = path.with_extension("tmp");
    if let Ok(bytes) = serde_json::to_vec_pretty(value) {
        if tokio::fs::write(&tmp, &bytes).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &path).await;
        }
    }
}

async fn write_json_at<T: serde::Serialize>(path: &Path, value: &T) {
    let _guard = STORE_LOCK.lock().await;
    write_json_at_unlocked(path, value).await;
}

async fn write_json<T: serde::Serialize>(rel: &[&str], value: &T) {
    write_json_at(&sub_dir(rel), value).await;
}

// ---------- config ----------

pub async fn load_config() -> Config {
    let mut cfg: Config = read_json(&["config.json"]).await.unwrap_or_default();
    if cfg.providers.is_empty() {
        cfg = default_config();
        save_config(&cfg).await;
    }
    cfg
}

fn default_config() -> Config {
    Config {
        active_provider: "openai".into(),
        providers: vec![
            crate::model::Provider {
                id: "openai".into(),
                name: "OpenAI 官方".into(),
                base_url: "https://api.openai.com/v1".into(),
                api_key: String::new(),
                models: vec![
                    "gpt-image-2".into(),
                    "gpt-image-2.5-sunburst".into(),
                    "gpt-image-2.5-flare".into(),
                ],
                overrides: Default::default(),
            },
            crate::model::Provider {
                id: "poke".into(),
                name: "Poke API".into(),
                base_url: "https://www.poke2api.com/v1".into(),
                api_key: String::new(),
                models: vec![
                    "gpt-image-2".into(),
                    "gpt-image-2.5-sunburst".into(),
                    "gpt-image-2.5-flare".into(),
                ],
                overrides: Default::default(),
            },
        ],
    }
}

pub async fn save_config(cfg: &Config) {
    write_json(&["config.json"], cfg).await;
}

// ---------- groups（节点图分组）----------

pub async fn list_groups() -> Vec<GraphGroup> {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    v.sort_by_key(|g| g.created_at);
    v
}

pub async fn save_group(group: &GraphGroup) {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    if let Some(slot) = v.iter_mut().find(|x| x.id == group.id) {
        *slot = group.clone();
    } else {
        v.push(group.clone());
    }
    write_json(&["groups.json"], &v).await;
}

pub async fn write_groups(groups: &[GraphGroup]) {
    write_json(&["groups.json"], &groups).await;
}

/// 删除分组，其中的图回到未分组
pub async fn delete_group(id: &str) {
    let mut v: Vec<GraphGroup> = read_json(&["groups.json"]).await.unwrap_or_default();
    v.retain(|x| x.id != id);
    write_json(&["groups.json"], &v).await;
    for graph in list_graphs().await {
        if graph.group_id.as_deref() == Some(id) {
            let mut g = graph;
            g.group_id = None;
            write_json(&["graphs", &g.id, "graph.json"], &g).await;
        }
    }
}

pub async fn set_graph_group(graph_id: &str, group_id: Option<String>) {
    let Some(mut graph) = get_graph(graph_id).await else {
        return;
    };
    graph.group_id = group_id;
    save_graph(&graph).await;
}

// ---------- graphs ----------

pub async fn list_graphs() -> Vec<Graph> {
    let mut out: Vec<Graph> = vec![];
    let root = sub_dir(&["graphs"]);
    let _ = tokio::fs::create_dir_all(&root).await;
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_dir()).unwrap_or(false) {
                let path = entry.path().join("graph.json");
                if let Some(graph) = read_json_at::<Graph>(&path).await {
                    if !graph.id.is_empty() {
                        out.push(graph);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    out
}

pub async fn get_graph(id: &str) -> Option<Graph> {
    read_json(&["graphs", id, "graph.json"]).await
}

pub async fn save_graph(graph: &Graph) {
    write_json(&["graphs", &graph.id, "graph.json"], graph).await;
}

pub async fn delete_graph(id: &str) {
    let _ = tokio::fs::remove_dir_all(sub_dir(&["graphs", id])).await;
}

// ---------- runs ----------

pub async fn list_runs(limit: usize) -> Vec<Run> {
    let mut out: Vec<Run> = vec![];
    let root = sub_dir(&["runs"]);
    if let Ok(mut rd) = tokio::fs::read_dir(&root).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            if entry.file_type().await.map(|t| t.is_file()).unwrap_or(false) {
                if let Some(run) = read_json_at::<Run>(&entry.path()).await {
                    if !run.id.is_empty() {
                        out.push(run);
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    out.truncate(limit);
    out
}

pub async fn get_run(id: &str) -> Option<Run> {
    read_json(&["runs", &format!("{id}.json")]).await
}

pub async fn save_run(run: &Run) {
    write_json(&["runs", &format!("{}.json", run.id)], run).await;
}

pub async fn delete_run(id: &str) {
    let _ = tokio::fs::remove_file(sub_dir(&["runs", &format!("{id}.json")])).await;
}

// ---------- assets ----------

pub fn new_asset_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// 保存资产字节，返回引用。ext 从文件名或字节嗅探。
pub async fn save_asset(bytes: &[u8], ext: &str) -> Result<AssetRef, String> {
    let id = new_asset_id();
    write_asset_file(&id, bytes, ext).await?;
    let (w, h) = sniff_dimensions(bytes);
    Ok(AssetRef {
        id,
        ext: ext.into(),
        w,
        h,
    })
}

async fn write_asset_file(id: &str, bytes: &[u8], ext: &str) -> Result<(), String> {
    let path = dir().join("assets").join(format!("{id}.{ext}"));
    if let Some(parent) = path.parent() {
        let _ = tokio::fs::create_dir_all(parent).await;
    }
    tokio::fs::write(&path, bytes)
        .await
        .map_err(|e| format!("写资产失败：{e}"))
}

pub async fn read_asset(asset: &AssetRef) -> Result<Vec<u8>, String> {
    let path = dir().join("assets").join(format!("{}.{}", asset.id, asset.ext));
    tokio::fs::read(&path)
        .await
        .map_err(|e| format!("读资产失败：{e}"))
}

/// GET /asset/{name} —— name 形如 {uuid}.{ext}
pub async fn serve_asset(
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Response {
    let Some((id, ext)) = name.rsplit_once('.') else {
        return (StatusCode::BAD_REQUEST, "bad asset name").into_response();
    };
    if !id.chars().all(|c| c.is_ascii_alphanumeric()) || ext.len() > 5 {
        return (StatusCode::BAD_REQUEST, "bad asset name").into_response();
    }
    let asset = AssetRef {
        id: id.into(),
        ext: ext.into(),
        w: None,
        h: None,
    };
    match read_asset(&asset).await {
        Ok(bytes) => {
            let mime = match ext {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                _ => "application/octet-stream",
            };
            (
                [
                    (header::CONTENT_TYPE, mime.to_string()),
                    (
                        header::CACHE_CONTROL,
                        "public, max-age=31536000, immutable".into(),
                    ),
                ],
                bytes,
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// 从 PNG / JPEG / WebP 字节里嗅探宽高（尽力而为）
fn sniff_dimensions(bytes: &[u8]) -> (Option<u32>, Option<u32>) {
    if bytes.len() > 24 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        return (Some(w), Some(h));
    }
    if bytes.len() > 3 && &bytes[0..3] == b"\xff\xd8\xff" {
        let mut i = 2;
        while i + 9 < bytes.len() {
            if bytes[i] != 0xFF {
                break;
            }
            let marker = bytes[i + 1];
            let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
            if (0xC0..=0xCF).contains(&marker)
                && marker != 0xC4
                && marker != 0xC8
                && marker != 0xCC
            {
                if i + 9 <= bytes.len() {
                    let h = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
                    let w = u16::from_be_bytes([bytes[i + 7], bytes[i + 8]]) as u32;
                    return (Some(w), Some(h));
                }
            }
            i += 2 + len;
        }
    }
    if bytes.len() > 30 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        if &bytes[12..16] == b"VP8X" && bytes.len() > 30 {
            let w = 1 + (bytes[24] as u32 | (bytes[25] as u32) << 8 | (bytes[26] as u32) << 16);
            let h = 1 + (bytes[27] as u32 | (bytes[28] as u32) << 8 | (bytes[29] as u32) << 16);
            return (Some(w), Some(h));
        }
    }
    (None, None)
}
