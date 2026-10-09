//! REST API（/api/*）。生成直接返回会话图片，不归档运行或产物。

use crate::model::*;
use crate::util::now_ms;
use axum::extract::{DefaultBodyLimit, Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

// ---------- 错误 ----------

#[derive(Debug)]
pub struct ApiError(pub StatusCode, pub String);

impl From<String> for ApiError {
    fn from(msg: String) -> Self {
        ApiError(StatusCode::BAD_REQUEST, msg)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

type ApiResult<T> = Result<T, ApiError>;

fn bad(msg: impl Into<String>) -> ApiError {
    ApiError(StatusCode::BAD_REQUEST, msg.into())
}

fn storage_error(msg: String) -> ApiError {
    ApiError(StatusCode::INTERNAL_SERVER_ERROR, msg)
}

// ---------- 路由 ----------

pub fn router() -> Router {
    Router::new()
        // Public provider discovery contains no credentials.
        .route("/backend", get(backend_info))
        .route("/providers", get(list_providers))
        .route(
            "/providers/{provider_id}/generate",
            post(generate).layer(DefaultBodyLimit::disable()),
        )
        // 节点图
        .route("/graphs", get(list_graphs).post(create_graph))
        .route(
            "/graphs/{id}",
            get(get_graph).put(update_graph).delete(delete_graph),
        )
        .route(
            "/graphs/{id}/view",
            get(get_graph_view).put(save_graph_view),
        )
        .route("/graphs/{id}/group", put(set_graph_group))
        .route("/graphs/{id}/title", put(rename_graph))
        // 分组
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", patch(rename_group).delete(delete_group))
        .route("/groups/{id}/parent", patch(move_group))
        // 资产
        .route("/assets", post(upload_asset))
        // 全局库（data/stores/，层级）
        .route("/stores", get(list_global_store).post(upload_global_store))
        .route("/stores/dirs", post(make_store_dir))
        .route(
            "/stores/{*path}",
            axum::routing::patch(move_store_path).delete(delete_store_path),
        )
        // 图私有 store（graphs/{gid}/store/，内联感知）
        .route(
            "/graphs/{id}/store",
            get(list_graph_store).post(upload_graph_store),
        )
        .route(
            "/graphs/{id}/store/{name}",
            axum::routing::delete(delete_graph_store_file),
        )
        // image-pipeline 模板还有这几个端点；尚未接入，先显式 501
        .route("/upscale", post(unimplemented))
        .route("/ip-adapter", post(unimplemented))
        .route("/style-transfer", post(unimplemented))
        .route("/generate-text", post(unimplemented))
}

async fn unimplemented() -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError(
        StatusCode::NOT_IMPLEMENTED,
        "该节点类型尚未接入本服务端".into(),
    ))
}

// ---------- 配置 ----------

async fn backend_info() -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(
        json!({ "id": crate::store::backend_id().await.map_err(storage_error)?, "capabilities": { "workspace": true, "providers": true } }),
    ))
}

async fn list_providers() -> Json<serde_json::Value> {
    let cfg = crate::store::load_config().await;
    Json(json!(cfg.providers.iter().map(|provider| {
        let profiles: BTreeMap<String, ModelProfile> = provider.models.iter().map(|model| {
            (model.clone(), crate::profiles::merged(model, provider.overrides.get(model)))
        }).collect();
        json!({ "id": provider.id, "name": provider.name, "models": provider.models, "profiles": profiles })
    }).collect::<Vec<_>>()))
}

// ---------- 节点图 ----------

async fn list_graphs() -> Json<Vec<Graph>> {
    Json(crate::store::list_graphs().await)
}

async fn get_graph(Path(id): Path<String>) -> ApiResult<Json<Graph>> {
    crate::store::get_graph(&id)
        .await
        .map(Json)
        .ok_or_else(|| bad("图不存在"))
}

/// 新建节点图：种入「Model + Prompt → Generate → Preview」最小闭环，
/// 并写默认布局视图。可指定目录与标题。
#[derive(Deserialize, Default)]
struct CreateGraphBody {
    #[serde(default)]
    group_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
}

async fn create_graph(body: Option<Json<CreateGraphBody>>) -> ApiResult<Json<Graph>> {
    let body = body.map(|Json(b)| b).unwrap_or_default();
    let now = now_ms();
    let cfg = crate::store::load_config().await;
    let (provider_id, model_id) = cfg
        .active()
        .map(|p| (p.id.clone(), p.models.first().cloned().unwrap_or_default()))
        .unwrap_or_default();
    if let Some(gid) = &body.group_id {
        if crate::store::list_groups()
            .await
            .iter()
            .all(|g| &g.id != gid)
        {
            return Err(bad("目标目录不存在"));
        }
    }
    let nid = |tag: &str| format!("{tag}-{}", uuid::Uuid::new_v4().simple());
    let (model_id_node, prompt_id, gen_id, prev_id) =
        (nid("model"), nid("prompt"), nid("gen"), nid("prev"));
    let title = body.title.unwrap_or_else(|| "未命名图".into());
    // Stable UUID identity is independent of the title.
    let graph_id = uuid::Uuid::new_v4().simple().to_string();
    let graph = Graph {
        id: graph_id.clone(),
        title,
        group_id: body.group_id,
        nodes: vec![
            GraphNode {
                id: model_id_node.clone(),
                r#type: NodeType::Model,
                params: serde_json::json!({
                    "provider": provider_id,
                    "modelId": model_id
                })
                .as_object()
                .unwrap()
                .clone()
                .into_iter()
                .collect(),
            },
            GraphNode {
                id: prompt_id.clone(),
                r#type: NodeType::Prompt,
                params: serde_json::json!({ "text": "a cat" })
                    .as_object()
                    .unwrap()
                    .clone()
                    .into_iter()
                    .collect(),
            },
            GraphNode {
                id: gen_id.clone(),
                r#type: NodeType::Generate,
                params: Default::default(),
            },
            GraphNode {
                id: prev_id.clone(),
                r#type: NodeType::Preview,
                params: Default::default(),
            },
        ],
        edges: vec![
            GraphEdge {
                id: format!("{model_id_node}:model->{gen_id}:model"),
                source: model_id_node.clone(),
                source_port: "model".into(),
                target: gen_id.clone(),
                target_port: "model".into(),
            },
            GraphEdge {
                id: format!("{prompt_id}:text->{gen_id}:prompt"),
                source: prompt_id.clone(),
                source_port: "text".into(),
                target: gen_id.clone(),
                target_port: "prompt".into(),
            },
            GraphEdge {
                id: format!("{gen_id}:image->{prev_id}:image"),
                source: gen_id.clone(),
                source_port: "image".into(),
                target: prev_id.clone(),
                target_port: "image".into(),
            },
        ],
        created_at: now,
        updated_at: now,
    };
    // 默认布局：输入列 / 生成列 / 预览列
    let view = crate::model::GraphView {
        positions: [
            (model_id_node, crate::model::Position { x: 60.0, y: 120.0 }),
            (prompt_id, crate::model::Position { x: 60.0, y: 380.0 }),
            (gen_id, crate::model::Position { x: 420.0, y: 200.0 }),
            (prev_id, crate::model::Position { x: 780.0, y: 200.0 }),
        ]
        .into_iter()
        .collect(),
        ..Default::default()
    };
    crate::store::save_view(&graph.id, &view)
        .await
        .map_err(storage_error)?;
    crate::store::save_graph(&graph)
        .await
        .map_err(storage_error)?;
    Ok(Json(graph))
}

/// PUT /api/graphs/{id} 的负载：只接受结构文档（节点 / 连线）。
/// 不接收 created_at/updated_at —— 簿记字段由服务端掌管，
/// 客户端快照也不应携带（此前要求整结构 Graph 导致前端 PUT 一律 422，
/// 画布结构实际从未保存成功）。
#[derive(Deserialize)]
struct GraphUpdateBody {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
}

/// 保存结构文档（节点/连线）。标题与分组是元数据，走专门接口/建图，
/// PUT 不覆盖，避免前端文档快照把元数据抹掉。
async fn update_graph(
    Path(id): Path<String>,
    Json(graph): Json<GraphUpdateBody>,
) -> ApiResult<Json<Graph>> {
    let Some(mut stored) = crate::store::get_graph(&id).await else {
        return Err(bad("图不存在"));
    };
    stored.nodes = graph.nodes;
    stored.edges = graph.edges;
    stored.updated_at = now_ms();
    crate::store::save_graph(&stored)
        .await
        .map_err(storage_error)?;
    Ok(Json(stored))
}

async fn delete_graph(Path(id): Path<String>) -> ApiResult<()> {
    crate::store::delete_graph(&id).await;
    Ok(())
}

/// 表现文档：GET 缺省返回空（首次保存前）
async fn get_graph_view(Path(id): Path<String>) -> Json<GraphView> {
    Json(crate::store::get_view(&id).await.unwrap_or_default())
}

/// 高频保存（拖动/视口），不推进图 updated_at
async fn save_graph_view(Path(id): Path<String>, Json(view): Json<GraphView>) -> ApiResult<()> {
    if crate::store::get_graph(&id).await.is_none() {
        return Err(bad("图不存在"));
    }
    crate::store::save_view(&id, &view)
        .await
        .map_err(storage_error)?;
    Ok(())
}

#[derive(Deserialize)]
struct GroupBody {
    group_id: Option<String>,
}

async fn set_graph_group(Path(id): Path<String>, Json(body): Json<GroupBody>) -> ApiResult<()> {
    crate::store::set_graph_group(&id, body.group_id)
        .await
        .map_err(storage_error)?;
    Ok(())
}

/// Titles do not change a graph's identity or its image references.
async fn rename_graph(Path(id): Path<String>, Json(body): Json<NameBody>) -> ApiResult<()> {
    let Some(mut graph) = crate::store::get_graph(&id).await else {
        return Err(bad("图不存在"));
    };
    graph.title = body.name;
    graph.updated_at = now_ms();
    crate::store::save_graph(&graph)
        .await
        .map_err(storage_error)?;
    Ok(())
}

// ---------- 分组 ----------

async fn list_groups() -> Json<Vec<GraphGroup>> {
    Json(crate::store::list_groups().await)
}

#[derive(Deserialize)]
struct NameBody {
    name: String,
}

#[derive(Deserialize)]
struct CreateDirBody {
    name: String,
    #[serde(default)]
    parent_id: Option<String>,
}

async fn create_group(Json(body): Json<CreateDirBody>) -> ApiResult<Json<GraphGroup>> {
    let groups = crate::store::list_groups().await;
    if let Some(pid) = &body.parent_id {
        if !groups.iter().any(|g| &g.id == pid) {
            return Err(bad("父目录不存在"));
        }
    }
    let group = GraphGroup {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: if body.name.trim().is_empty() {
            "新建文件夹".into()
        } else {
            body.name
        },
        parent_id: body.parent_id,
        created_at: now_ms(),
    };
    crate::store::save_group(&group).await;
    Ok(Json(group))
}

async fn rename_group(Path(id): Path<String>, Json(body): Json<NameBody>) -> ApiResult<()> {
    let mut groups = crate::store::list_groups().await;
    if let Some(g) = groups.iter_mut().find(|g| g.id == id) {
        g.name = body.name;
    }
    crate::store::write_groups(&groups).await;
    Ok(())
}

#[derive(Deserialize)]
struct MoveBody {
    parent_id: Option<String>,
}

/// 移动目录到目标父目录（None = 根）。目标不能是自己或自己的后代。
async fn move_group(Path(id): Path<String>, Json(body): Json<MoveBody>) -> ApiResult<()> {
    let mut groups = crate::store::list_groups().await;
    if let Some(pid) = &body.parent_id {
        // 沿 parent 链向上走，若路过自己则成环
        let mut cursor = pid.clone();
        for _ in 0..groups.len() + 1 {
            if cursor == id {
                return Err(bad("不能把目录移动到它自己内部"));
            }
            let Some(parent) = groups.iter().find(|g| g.id == cursor) else {
                return Err(bad("目标目录不存在"));
            };
            match &parent.parent_id {
                Some(next) => cursor = next.clone(),
                None => break,
            }
        }
        if !groups.iter().any(|g| &g.id == pid) {
            return Err(bad("目标目录不存在"));
        }
    }
    if let Some(g) = groups.iter_mut().find(|g| g.id == id) {
        g.parent_id = body.parent_id;
    }
    crate::store::write_groups(&groups).await;
    Ok(())
}

async fn delete_group(Path(id): Path<String>) -> ApiResult<()> {
    crate::store::delete_group(&id).await;
    Ok(())
}

// ---------- 资产 ----------

#[derive(Deserialize)]
struct UploadQuery {
    filename: String,
}

/// POST /api/assets?filename=foo.png —— 原始字节体。
async fn upload_asset(
    Query(q): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<AssetRef>> {
    upload_asset_inner(q.filename, &body).await.map(Json)
}

async fn upload_asset_inner(filename: String, bytes: &[u8]) -> ApiResult<AssetRef> {
    if bytes.is_empty() {
        return Err(bad("空文件"));
    }
    let ext = filename.rsplit('.').next().unwrap_or("png").to_lowercase();
    let ext = match ext.as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "gif" => {
            if ext == "jpeg" {
                "jpg".to_string()
            } else {
                ext
            }
        }
        _ => "png".to_string(),
    };
    crate::store::save_asset(bytes, &ext).await.map_err(bad)
}

// ---------- image store（两层：全局库 + 图私有） ----------

/// 全局库全树（data/stores/，层级）
async fn list_global_store() -> Json<crate::store::StoreTree> {
    Json(crate::store::list_global_store().await)
}

#[derive(Deserialize)]
struct StoreUploadQuery {
    filename: String,
    #[serde(default)]
    dir: String,
}

/// 上传到全局库指定子目录（result 图拖入收藏 / 手动上传）
async fn upload_global_store(
    Query(q): Query<StoreUploadQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<crate::store::StoreFileEntry>> {
    crate::store::save_global_store_file(&q.dir, &q.filename, &body)
        .await
        .map(Json)
        .map_err(bad)
}

#[derive(Deserialize)]
struct StoreDirBody {
    path: String,
}

/// 新建文件夹（可多级 "角色/猫"）
async fn make_store_dir(Json(b): Json<StoreDirBody>) -> ApiResult<()> {
    crate::store::make_global_store_dir(&b.path)
        .await
        .map_err(bad)
}

#[derive(Deserialize)]
struct StoreMoveBody {
    to: String,
}

/// 重命名 / 移动（文件或目录；path 为相对路径）
async fn move_store_path(Path(path): Path<String>, Json(b): Json<StoreMoveBody>) -> ApiResult<()> {
    crate::store::move_global_store_path(&path, &b.to)
        .await
        .map_err(bad)
}

/// 删除文件或目录（目录递归；path 为相对路径）
async fn delete_store_path(Path(path): Path<String>) -> ApiResult<()> {
    crate::store::delete_global_store_file(&path)
        .await
        .map_err(bad)
}

/// 图私有 store 列表（graphs/{gid}/store/）
async fn list_graph_store(Path(id): Path<String>) -> ApiResult<Json<Vec<crate::store::StoreFile>>> {
    if crate::store::get_graph(&id).await.is_none() {
        return Err(bad("图不存在"));
    }
    Ok(Json(crate::store::list_graph_store(&id).await))
}

/// 上传到图私有 store（LoadImage 上传 / 从库拖入时复制进来）
async fn upload_graph_store(
    Path(id): Path<String>,
    Query(q): Query<StoreUploadQuery>,
    body: axum::body::Bytes,
) -> ApiResult<Json<crate::store::StoreFile>> {
    if crate::store::get_graph(&id).await.is_none() {
        return Err(bad("图不存在"));
    }
    crate::store::save_graph_store_file(&id, &q.filename, &body)
        .await
        .map(Json)
        .map_err(bad)
}

async fn delete_graph_store_file(Path((id, name)): Path<(String, String)>) -> ApiResult<()> {
    crate::store::delete_graph_store_file(&id, &name)
        .await
        .map_err(bad)
}

// ---------- 会话内生成 ----------

/// Generate 节点请求体。参数为统一键 → 标量值，按模型档案校验和映射。
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GenerateParams {
    model: String,
    prompt: String,
    #[serde(default)]
    params: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    image_urls: Vec<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateResult {
    image_urls: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<Usage>,
}

/// 读取本次参考图，不创建永久资产副本。
async fn read_input_image(url: &str) -> ApiResult<crate::adapter::InputImage> {
    use crate::adapter::InputImage;
    use percent_encoding::percent_decode_str;
    let decode = |value: &str| -> ApiResult<String> {
        percent_decode_str(value)
            .decode_utf8()
            .map(|s| s.into_owned())
            .map_err(|_| bad("图片 URL 编码不合法"))
    };
    let ext = |filename: &str| {
        filename
            .rsplit_once('.')
            .map(|(_, e)| e.to_ascii_lowercase())
            .unwrap_or_else(|| "png".into())
    };
    if let Some(rest) = url.strip_prefix("/gstore/") {
        let (gid, name) = rest
            .split_once('/')
            .ok_or_else(|| bad("图 store URL 格式不对"))?;
        let gid = decode(gid)?;
        if gid.is_empty() || gid == "." || gid == ".." || gid.contains('/') || gid.contains('\\') {
            return Err(bad("图 store URL 格式不对"));
        }
        let name = decode(name)?;
        let bytes = crate::store::read_graph_store_file(&gid, &name)
            .await
            .map_err(bad)?;
        return Ok(InputImage {
            bytes,
            ext: ext(&name),
        });
    }
    if let Some(rest) = url.strip_prefix("/store/") {
        let path = decode(rest)?;
        let path =
            crate::store::safe_store_path(&path).ok_or_else(|| bad("库图片 URL 格式不对"))?;
        let bytes = tokio::fs::read(crate::store::sub_dir(&["stores", &path]))
            .await
            .map_err(|e| bad(format!("读取库图片失败：{e}")))?;
        return Ok(InputImage {
            bytes,
            ext: ext(&path),
        });
    }
    // 已有用户上传资产仍可作为输入；不在生成时新增资产。
    if let Some(name) = url.strip_prefix("/asset/") {
        let (id, ext) = name
            .rsplit_once('.')
            .ok_or_else(|| bad("资产 URL 格式不对"))?;
        if id.is_empty()
            || !id.chars().all(|c| c.is_ascii_alphanumeric())
            || !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext)
        {
            return Err(bad("资产 URL 格式不对"));
        }
        let asset = AssetRef {
            id: id.into(),
            ext: ext.into(),
            w: None,
            h: None,
        };
        return Ok(InputImage {
            bytes: crate::store::read_asset(&asset).await.map_err(bad)?,
            ext: ext.into(),
        });
    }
    if let Some(rest) = url.strip_prefix("data:") {
        let (meta, b64) = rest
            .split_once(',')
            .ok_or_else(|| bad("data URL 格式不对"))?;
        let mime = meta
            .strip_suffix(";base64")
            .ok_or_else(|| bad("参考图需要 base64 data URL"))?;
        let ext = match mime {
            "image/png" => "png",
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            "image/gif" => "gif",
            _ => return Err(bad("参考图类型不支持")),
        };
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| bad(format!("data URL 解码失败：{e}")))?;
        return Ok(InputImage {
            bytes,
            ext: ext.into(),
        });
    }
    Err(bad(
        "仅支持库图片、图内图片、已有资产与 data: 图片；外部 URL 请先用图片节点上传",
    ))
}

/// The provider is selected explicitly by the route, independent of graph storage.
async fn generate(
    Path(provider_id): Path<String>,
    Json(p): Json<GenerateParams>,
) -> ApiResult<Json<GenerateResult>> {
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == provider_id)
        .ok_or_else(|| bad("Provider 不存在"))?;
    let model_id = p.model;
    if !provider.models.contains(&model_id) {
        return Err(bad("Provider 未配置该模型"));
    }
    let profile = crate::profiles::merged(&model_id, provider.overrides.get(&model_id));
    if p.prompt.trim().is_empty() {
        return Err(bad("Prompt 为空"));
    }
    let mut params: ParamMap = BTreeMap::new();
    for (key, value) in p.params {
        let v = match value {
            serde_json::Value::String(s) => ParamValue::Text(s),
            serde_json::Value::Number(n) => match n.as_f64() {
                Some(f) => ParamValue::Number(f),
                None => continue,
            },
            serde_json::Value::Bool(b) => ParamValue::Text(b.to_string()),
            _ => continue,
        };
        params.insert(key, v);
    }
    let params = with_defaults(&profile, &params);
    let mut images = Vec::new();
    for url in &p.image_urls {
        if !url.is_empty() {
            images.push(read_input_image(url).await?);
        }
    }
    validate_request(&profile, &model_id, &params, images.len()).map_err(bad)?;
    let outcome = crate::adapter::execute(
        provider,
        crate::adapter::ExecRequest {
            prompt: &p.prompt,
            params: &params,
            images: &images,
            model_id: &model_id,
            profile: &profile,
        },
    )
    .await
    .map_err(|e| ApiError(StatusCode::BAD_GATEWAY, e))?;
    Ok(Json(GenerateResult {
        image_urls: outcome.image_urls,
        usage: outcome.usage,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Bytes,
        http::{HeaderMap, Uri},
    };

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nmock-image";

    async fn mock_provider(
        response: serde_json::Value,
        status: StatusCode,
    ) -> (
        String,
        tokio::sync::mpsc::UnboundedReceiver<(String, HeaderMap, Vec<u8>)>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let response = response.to_string().replace("MOCK_BASE", &base);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = move |uri: Uri, headers: HeaderMap, bytes: Bytes| {
            let tx = tx.clone();
            let response = response.clone();
            async move {
                tx.send((uri.path().to_string(), headers, bytes.to_vec()))
                    .unwrap();
                (status, [("content-type", "application/json")], response)
            }
        };
        let app = Router::new()
            .route("/images/generations", post(handler.clone()))
            .route("/images/edits", post(handler))
            .route(
                "/image.png",
                get(|| async { ([("content-type", "image/png")], PNG) }),
            )
            .layer(DefaultBodyLimit::disable());
        let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
        (base, rx, task)
    }

    async fn configure(base: &str) {
        crate::store::save_config(&Config {
            active_provider: "mock".into(),
            providers: vec![Provider {
                id: "mock".into(),
                name: "mock".into(),
                base_url: base.into(),
                api_key: "sk-local-test".into(),
                models: vec!["gpt-image-2".into()],
                overrides: Default::default(),
            }],
        })
        .await;
    }

    fn request(images: Vec<String>) -> Json<GenerateParams> {
        Json(GenerateParams {
            model: "gpt-image-2".into(),
            prompt: "a cat".into(),
            params: BTreeMap::new(),
            image_urls: images,
        })
    }

    fn temporary_url() -> String {
        format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(PNG)
        )
    }

    #[tokio::test]
    async fn provider_discovery_exposes_capabilities_without_credentials() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("provider-discovery");
        configure("https://upstream.example/v1").await;
        let identity = crate::store::backend_id().await.unwrap();
        assert_eq!(crate::store::backend_id().await.unwrap(), identity);
        assert_eq!(backend_info().await.unwrap().0["id"], identity);
        let Json(public) = list_providers().await;
        assert_eq!(public[0]["id"], "mock");
        assert_eq!(public[0]["profiles"]["gpt-image-2"]["id"], "gpt-image-2");
        let text = public.to_string();
        assert!(!text.contains("sk-local-test"));
        assert!(public[0].get("api_key").is_none());
        assert!(!text.contains("upstream.example"));
        assert!(generate(Path("missing".into()), request(vec![]))
            .await
            .is_err());
        let mut body = request(vec![]);
        body.model = "unconfigured".into();
        assert!(generate(Path("mock".into()), body).await.is_err());
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn graph_titles_do_not_change_identity_or_reference_files() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("stable-graph");
        let Json(graph) = create_graph(None).await.unwrap();
        assert!(uuid::Uuid::parse_str(&graph.id).is_ok());
        crate::store::save_graph_store_file(&graph.id, "reference.png", PNG)
            .await
            .unwrap();
        rename_graph(
            Path(graph.id.clone()),
            Json(NameBody {
                name: "新的 / 标题".into(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(
            crate::store::read_graph_store_file(&graph.id, "reference.png")
                .await
                .unwrap(),
            PNG
        );
        assert_eq!(
            crate::store::get_graph(&graph.id).await.unwrap().title,
            "新的 / 标题"
        );
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn generate_returns_all_temporary_results_without_archives() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("generate");
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG);
        let (base, mut requests, task) = mock_provider(
            json!({
                "data": [{"b64_json": b64}, {"url": "MOCK_BASE/image.png"}],
                "usage": {"total_tokens": 42}
            }),
            StatusCode::OK,
        )
        .await;
        configure(&base).await;
        let Json(result) = generate(Path("mock".into()), request(vec![]))
            .await
            .unwrap();
        assert_eq!(result.image_urls, vec![temporary_url(), temporary_url()]);
        assert_eq!(result.usage.unwrap().total_tokens, Some(42));
        let (path, headers, body) = requests.recv().await.unwrap();
        assert_eq!(path, "/images/generations");
        assert_eq!(headers["authorization"], "Bearer sk-local-test");
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["prompt"], "a cat");
        assert_eq!(body["model"], "gpt-image-2");
        assert!(!tmp.join("runs").exists());
        assert!(!tmp.join("assets").exists());
        assert!(!tmp.join("stores").exists());
        task.abort();
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn edits_reads_encoded_references_and_temporary_results_without_copying() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("edits");
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG);
        let (base, mut requests, task) =
            mock_provider(json!({"data": [{"b64_json": b64}]}), StatusCode::OK).await;
        configure(&base).await;
        crate::store::save_graph_store_file("未命名图", "参考.png", PNG)
            .await
            .unwrap();
        crate::store::save_global_store_file("库", "猫.png", PNG)
            .await
            .unwrap();
        let images = vec![
            "/gstore/%E6%9C%AA%E5%91%BD%E5%90%8D%E5%9B%BE/%E5%8F%82%E8%80%83.png".into(),
            "/store/%E5%BA%93/%E7%8C%AB.png".into(),
            temporary_url(),
        ];
        let Json(result) = generate(Path("mock".into()), request(images))
            .await
            .unwrap();
        assert_eq!(result.image_urls, vec![temporary_url()]);
        let (path, headers, body) = requests.recv().await.unwrap();
        assert_eq!(path, "/images/edits");
        assert!(headers["content-type"]
            .to_str()
            .unwrap()
            .starts_with("multipart/form-data; boundary="));
        let text = String::from_utf8_lossy(&body);
        assert_eq!(text.matches("name=\"image[]\"").count(), 3);
        assert!(body.windows(PNG.len()).any(|w| w == PNG));
        assert!(!tmp.join("runs").exists());
        assert!(!tmp.join("assets").exists());
        task.abort();
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn generation_errors_and_removed_history_routes() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("generate-error");
        let (base, mut requests, task) = mock_provider(
            json!({"error": {"message": "mock rejected"}}),
            StatusCode::BAD_REQUEST,
        )
        .await;
        configure(&base).await;
        let error = generate(Path("mock".into()), request(vec![temporary_url()]))
            .await
            .err()
            .unwrap();
        assert_eq!(error.0, StatusCode::BAD_GATEWAY);
        assert_eq!(error.1, "mock rejected");
        assert_eq!(requests.recv().await.unwrap().0, "/images/edits");
        assert!(!tmp.join("runs").exists());
        assert!(!tmp.join("assets").exists());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, router()).await.unwrap() });
        let client = reqwest::Client::new();
        assert_eq!(
            client
                .get(format!("{local}/runs"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            client
                .post(format!("{local}/runs/old/rerun"))
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        server.abort();
        task.abort();
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn chained_image_larger_than_default_body_limit_can_be_generated() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("large-input");
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG);
        let (base, mut requests, upstream) =
            mock_provider(json!({"data": [{"b64_json": b64}]}), StatusCode::OK).await;
        configure(&base).await;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move { axum::serve(listener, router()).await.unwrap() });
        let image = format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(vec![0; 2 * 1024 * 1024])
        );
        let response = reqwest::Client::new()
            .post(format!("{local}/providers/mock/generate"))
            .json(&json!({
                "model": "gpt-image-2", "prompt": "a cat", "imageUrls": [image]
            }))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(requests.recv().await.unwrap().0, "/images/edits");
        assert!(!tmp.join("runs").exists());
        assert!(!tmp.join("assets").exists());
        server.abort();
        upstream.abort();
        std::fs::remove_dir_all(tmp).unwrap();
    }

    #[tokio::test]
    async fn graph_and_view_write_failures_surface_as_errors() {
        let (tmp, _guard) = crate::store::store_tests::use_tmp("save-errors");
        let Json(graph) = create_graph(None).await.unwrap();
        let graph_root = tmp.join("graphs").join(&graph.id);
        std::fs::create_dir(graph_root.join("graph.tmp")).unwrap();
        let error = update_graph(
            Path(graph.id.clone()),
            Json(GraphUpdateBody {
                nodes: vec![],
                edges: vec![],
            }),
        )
        .await
        .err()
        .unwrap();
        assert_eq!(error.0, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(!crate::store::get_graph(&graph.id)
            .await
            .unwrap()
            .nodes
            .is_empty());
        std::fs::create_dir(graph_root.join("view.tmp")).unwrap();
        let view: GraphView =
            serde_json::from_value(json!({"outputs": {"old": "/asset/old.png"}})).unwrap();
        assert!(serde_json::to_value(&view)
            .unwrap()
            .get("outputs")
            .is_none());
        let error = save_graph_view(Path(graph.id), Json(view))
            .await
            .err()
            .unwrap();
        assert_eq!(error.0, StatusCode::INTERNAL_SERVER_ERROR);
        std::fs::remove_dir_all(tmp).unwrap();
    }
}
