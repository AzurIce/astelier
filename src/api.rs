//! REST API（/api/*）—— React 前端（web/）与服务端之间的全部接口。
//!
//! 约定：
//! - JSON in/out；错误统一 `{ "error": "..." }` + 恰当的状态码；
//! - 生图一律先落 Run 档案再异步执行（`persist_and_execute`），
//!   `/api/runs` 轮询状态；`/api/generate` 是给节点图前端的同步封装
//!   （内部同样走 Run，等它完成再返回）。

use crate::model::*;
use crate::util::now_ms;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post, put};
use axum::{Json, Router};
use base64::Engine as _;
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;

// ---------- 错误 ----------

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

// ---------- 路由 ----------

pub fn router() -> Router {
    Router::new()
        // 配置
        .route("/config", get(get_config).put(save_config))
        .route(
            "/providers/{provider_id}/profiles",
            get(resolve_profiles),
        )
        .route(
            "/providers/{provider_id}/models/{model_id}/override",
            put(set_model_override),
        )
        // 节点图
        .route("/graphs", get(list_graphs).post(create_graph))
        .route(
            "/graphs/{id}",
            get(get_graph).put(update_graph).delete(delete_graph),
        )
        .route("/graphs/{id}/group", put(set_graph_group))
        // 分组
        .route("/groups", get(list_groups).post(create_group))
        .route("/groups/{id}", patch(rename_group).delete(delete_group))
        // 资产
        .route("/assets", post(upload_asset))
        // 批次
        .route("/runs", get(list_runs).post(start_run))
        .route(
            "/runs/{id}",
            get(get_run).delete(delete_run),
        )
        .route("/runs/{id}/rerun", post(rerun_run))
        // 节点图前端（tldraw image-pipeline）的同步端点
        .route("/generate", post(generate))
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

async fn get_config() -> Json<Config> {
    Json(crate::store::load_config().await)
}

async fn save_config(Json(cfg): Json<Config>) -> ApiResult<()> {
    crate::store::save_config(&cfg).await;
    Ok(())
}

/// provider 的全部模型档案（内置 + override 合并）
async fn resolve_profiles(Path(provider_id): Path<String>) -> Json<Vec<ModelProfile>> {
    let cfg = crate::store::load_config().await;
    let Some(provider) = cfg.providers.iter().find(|p| p.id == provider_id) else {
        return Json(vec![]);
    };
    Json(
        provider
            .models
            .iter()
            .map(|m| crate::profiles::merged(m, provider.overrides.get(m)))
            .collect(),
    )
}

async fn set_model_override(
    Path((provider_id, model_id)): Path<(String, String)>,
    Json(override_json): Json<Option<serde_json::Value>>,
) -> ApiResult<()> {
    let mut cfg = crate::store::load_config().await;
    if let Some(p) = cfg.providers.iter_mut().find(|p| p.id == provider_id) {
        match override_json {
            None | Some(serde_json::Value::Null) => {
                p.overrides.remove(&model_id);
            }
            Some(v) => {
                p.overrides.insert(model_id, v);
            }
        }
        crate::store::save_config(&cfg).await;
    }
    Ok(())
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

/// 新建节点图：种入「生图 → 显示」最小闭环。
async fn create_graph() -> ApiResult<Json<Graph>> {
    let now = now_ms();
    let cfg = crate::store::load_config().await;
    let (provider_id, model_id) = cfg
        .active()
        .map(|p| (p.id.clone(), p.models.first().cloned().unwrap_or_default()))
        .unwrap_or_default();
    let gen_id = format!("gen-{}", uuid::Uuid::new_v4().simple());
    let disp_id = format!("disp-{}", uuid::Uuid::new_v4().simple());
    let graph = Graph {
        id: uuid::Uuid::new_v4().simple().to_string(),
        title: "未命名图".into(),
        group_id: None,
        nodes: vec![
            GraphNode {
                id: gen_id.clone(),
                x: 80.0,
                y: 160.0,
                data: NodeData::Gen(GenNodeData {
                    provider_id,
                    model_id,
                    prompt: String::new(),
                    params: Default::default(),
                }),
            },
            GraphNode {
                id: disp_id.clone(),
                x: 480.0,
                y: 160.0,
                data: NodeData::Display(DisplayNodeData::default()),
            },
        ],
        edges: vec![GraphEdge {
            id: format!("e-{}", uuid::Uuid::new_v4().simple()),
            source: gen_id,
            target: disp_id,
            source_handle: None,
            target_handle: None,
        }],
        created_at: now,
        updated_at: now,
    };
    crate::store::save_graph(&graph).await;
    Ok(Json(graph))
}

async fn update_graph(Path(id): Path<String>, Json(mut graph): Json<Graph>) -> ApiResult<Json<Graph>> {
    if graph.id != id {
        return Err(bad("路径与体内的图 id 不一致"));
    }
    graph.updated_at = now_ms();
    crate::store::save_graph(&graph).await;
    Ok(Json(graph))
}

async fn delete_graph(Path(id): Path<String>) -> ApiResult<()> {
    crate::store::delete_graph(&id).await;
    Ok(())
}

#[derive(Deserialize)]
struct GroupBody {
    group_id: Option<String>,
}

async fn set_graph_group(Path(id): Path<String>, Json(body): Json<GroupBody>) -> ApiResult<()> {
    crate::store::set_graph_group(&id, body.group_id).await;
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

async fn create_group(Json(body): Json<NameBody>) -> Json<GraphGroup> {
    let group = GraphGroup {
        id: uuid::Uuid::new_v4().simple().to_string(),
        name: if body.name.trim().is_empty() {
            "新建分组".into()
        } else {
            body.name
        },
        created_at: now_ms(),
    };
    crate::store::save_group(&group).await;
    Json(group)
}

async fn rename_group(Path(id): Path<String>, Json(body): Json<NameBody>) -> ApiResult<()> {
    let mut groups = crate::store::list_groups().await;
    if let Some(g) = groups.iter_mut().find(|g| g.id == id) {
        g.name = body.name;
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
            if ext == "jpeg" { "jpg".to_string() } else { ext }
        }
        _ => "png".to_string(),
    };
    crate::store::save_asset(bytes, &ext)
        .await
        .map_err(bad)
}

// ---------- 执行（Run 档案 + 异步执行）----------

/// 落盘批次并异步执行最终请求。
async fn persist_and_execute(
    provider: Provider,
    profile: ModelProfile,
    resolved: ResolvedRequest,
    run: Run,
) -> Run {
    crate::store::save_run(&run).await;
    let mut run_task = run.clone();
    tokio::spawn(async move {
        let t0 = std::time::Instant::now();
        match crate::adapter::execute(
            &provider,
            crate::adapter::ExecRequest {
                prompt: &resolved.prompt,
                params: &resolved.params,
                images: &resolved.images,
                mask: resolved.mask.as_ref(),
                model_id: &run_task.model_id,
                profile: &profile,
            },
        )
        .await
        {
            Ok(outcome) => {
                run_task.status = RunStatus::Done;
                run_task.images = outcome.images;
                run_task.usage = outcome.usage;
            }
            Err((status, msg)) => {
                run_task.status = status;
                run_task.error = Some(msg);
            }
        }
        run_task.duration_ms = Some(t0.elapsed().as_millis() as u64);
        crate::store::save_run(&run_task).await;
    });
    run
}

fn new_run(
    provider_id: String,
    model_id: String,
    resolved: ResolvedRequest,
    graph_id: Option<String>,
    node_id: Option<String>,
    rerun_of: Option<String>,
) -> Run {
    Run {
        id: uuid::Uuid::new_v4().simple().to_string(),
        recipe_id: String::new(),
        input_id: None,
        recipe_version: 0,
        input_version: 0,
        provider_id,
        model_id,
        mode: resolved.mode,
        request: None,
        graph_id,
        node_id,
        resolved: Some(resolved),
        rerun_of,
        status: RunStatus::Running,
        error: None,
        images: vec![],
        usage: None,
        created_at: now_ms(),
        duration_ms: None,
        prompt: String::new(),
        params: Default::default(),
        ref_count: 0,
    }
}

/// 生图请求 = 发给 API 的全部内容（不带 Run 簿记字段）。
#[derive(Deserialize, Debug)]
pub struct RunBody {
    pub provider_id: String,
    pub model_id: String,
    pub prompt: String,
    #[serde(default)]
    pub params: ParamMap,
    #[serde(default)]
    pub images: Vec<AssetRef>,
    #[serde(default)]
    pub mask: Option<AssetRef>,
    #[serde(default)]
    pub graph_id: Option<String>,
    #[serde(default)]
    pub node_id: Option<String>,
}

/// 组装校验 + 起跑一条 Run（内部供 /api/runs、/api/runs/{id}/rerun 与
/// /api/generate 共用）。
async fn launch_run(body: RunBody, rerun_of: Option<String>) -> ApiResult<Run> {
    let cfg = crate::store::load_config().await;
    let provider = cfg
        .providers
        .iter()
        .find(|p| p.id == body.provider_id)
        .cloned()
        .ok_or_else(|| bad("Provider 不存在"))?;
    let profile = crate::profiles::merged(&body.model_id, provider.overrides.get(&body.model_id));

    let mode = if body.images.is_empty() { Mode::Gen } else { Mode::Edit };
    let resolved = ResolvedRequest {
        prompt: body.prompt,
        params: body.params,
        images: body.images,
        mask: body.mask,
        mode,
    };
    if resolved.prompt.trim().is_empty() {
        return Err(bad("Prompt 为空"));
    }
    crate::model::validate_request(&profile, &body.model_id, &resolved.params, resolved.images.len())
        .map_err(bad)?;

    let run = new_run(
        provider.id.clone(),
        body.model_id,
        resolved.clone(),
        body.graph_id,
        body.node_id,
        rerun_of,
    );
    Ok(persist_and_execute(provider, profile, resolved, run).await)
}

async fn start_run(Json(body): Json<RunBody>) -> ApiResult<Json<Run>> {
    launch_run(body, None).await.map(Json)
}

#[derive(Deserialize)]
struct ListRunsQuery {
    #[serde(default = "default_run_limit")]
    limit: usize,
}

fn default_run_limit() -> usize {
    50
}

async fn list_runs(Query(q): Query<ListRunsQuery>) -> Json<Vec<Run>> {
    Json(crate::store::list_runs(q.limit).await)
}

async fn get_run(Path(id): Path<String>) -> ApiResult<Json<Run>> {
    crate::store::get_run(&id).await.map(Json).ok_or_else(|| bad("批次不存在"))
}

/// 原样重放：按批次归档的最终请求再执行一次。
async fn rerun_run(Path(id): Path<String>) -> ApiResult<Json<Run>> {
    let old = crate::store::get_run(&id)
        .await
        .ok_or_else(|| bad("批次不存在"))?;
    // 新批次一律有 resolved；旧配方批次回放其快照里的请求
    let (resolved, provider_id, model_id, graph_id, node_id) = if let Some(r) = old.resolved {
        (r, old.provider_id, old.model_id, old.graph_id, old.node_id)
    } else if let Some(request) = old.request {
        (
            request.resolved,
            request.template.provider_id,
            request.template.model_id,
            None,
            None,
        )
    } else {
        return Err(bad("该批次创建于快照机制之前，没有可重放的请求归档"));
    };
    let run = launch_run(
        RunBody {
            provider_id,
            model_id,
            prompt: resolved.prompt.clone(),
            params: resolved.params.clone(),
            images: resolved.images.clone(),
            mask: resolved.mask.clone(),
            graph_id,
            node_id,
        },
        Some(old.id),
    )
    .await?;
    Ok(Json(run))
}

async fn delete_run(Path(id): Path<String>) -> ApiResult<()> {
    crate::store::delete_run(&id).await;
    Ok(())
}

// ---------- /api/generate：tldraw 节点图前端的同步封装 ----------

/// image-pipeline 前端 Generate 节点的请求体（camelCase，与模板一致）。
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GenerateParams {
    model: String,
    prompt: String,
    negative_prompt: Option<String>,
    steps: Option<f64>,
    cfg_scale: Option<f64>,
    seed: Option<f64>,
    reference_image_url: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerateResult {
    image_url: String,
    seed: f64,
}

/// 前端图片值（LoadImage 的 data URL / 本服务生成的 /asset/ 相对路径）→ AssetRef。
async fn url_to_asset(url: &str) -> ApiResult<AssetRef> {
    if let Some(name) = url.strip_prefix("/asset/") {
        let (id, ext) = name
            .rsplit_once('.')
            .ok_or_else(|| bad("资产 URL 格式不对"))?;
        if !id.chars().all(|c| c.is_ascii_alphanumeric()) || ext.len() > 5 {
            return Err(bad("资产 URL 格式不对"));
        }
        return Ok(AssetRef {
            id: id.into(),
            ext: ext.into(),
            w: None,
            h: None,
        });
    }
    if let Some(rest) = url.strip_prefix("data:") {
        let (meta, b64) = rest.split_once(',').ok_or_else(|| bad("data URL 格式不对"))?;
        let ext = meta
            .strip_prefix("image/")
            .and_then(|s| s.split(';').next())
            .map(|s| if s == "jpeg" { "jpg" } else { s })
            .unwrap_or("png")
            .to_string();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| bad(format!("data URL 解码失败：{e}")))?;
        return crate::store::save_asset(&bytes, &ext).await.map_err(bad);
    }
    Err(bad(
        "仅支持本服务资产（/asset/…）与 data: 图片；外部 URL 请先用图片节点上传",
    ))
}

/// model 字符串解析：`provider:model`（前缀命中已配置 provider 时拆开），
/// 否则整体视作 model_id、走当前激活的 provider。
async fn resolve_model(cfg: &Config, model: &str) -> ApiResult<(String, String)> {
    if let Some((pid, mid)) = model.split_once(':') {
        if cfg.providers.iter().any(|p| p.id == pid) {
            return Ok((pid.to_string(), mid.to_string()));
        }
    }
    let active = cfg.active().ok_or_else(|| bad("没有可用的 Provider，请先在设置里配置"))?;
    Ok((active.id.clone(), model.to_string()))
}

/// 同步生成：内部照样走 Run（可追溯、可重放），轮询直到完成。
/// 上游生图常需几十秒，client 超时请留足（前端 fetch 默认即可）。
async fn generate(Json(p): Json<GenerateParams>) -> ApiResult<Json<GenerateResult>> {
    let cfg = crate::store::load_config().await;
    let (provider_id, model_id) = resolve_model(&cfg, &p.model).await?;

    // 负面提示：OpenAI Images 协议无独立字段，拼进 prompt 尾部
    let prompt = match p.negative_prompt.as_deref().map(str::trim) {
        Some(n) if !n.is_empty() => format!("{}\n\n(avoid: {})", p.prompt, n),
        _ => p.prompt,
    };

    let mut params: ParamMap = BTreeMap::new();
    if let Some(steps) = p.steps {
        params.insert("steps".into(), ParamValue::Number(steps));
    }
    if let Some(cfg_scale) = p.cfg_scale {
        params.insert("cfg_scale".into(), ParamValue::Number(cfg_scale));
    }
    if let Some(seed) = p.seed {
        params.insert("seed".into(), ParamValue::Number(seed));
    }
    let seed = p.seed.unwrap_or(0.0);

    let images = match p.reference_image_url.as_deref() {
        Some(u) if !u.is_empty() => vec![url_to_asset(u).await?],
        _ => vec![],
    };

    let run = launch_run(RunBody {
        provider_id,
        model_id,
        prompt,
        params,
        images,
        mask: None,
        graph_id: None,
        node_id: None,
    }, None)
    .await?;

    // 轮询批次完成（adapter 自身有 600s 超时，这里等 300s 足够）
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    loop {
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
        let run = crate::store::get_run(&run.id)
            .await
            .ok_or_else(|| bad("批次档案丢失"))?;
        match run.status {
            RunStatus::Done => {
                let url = run
                    .images
                    .first()
                    .map(|a| a.url())
                    .ok_or_else(|| bad("生成完成但没有产出图片"))?;
                return Ok(Json(GenerateResult { image_url: url, seed }));
            }
            RunStatus::Error => {
                let msg = run.error.unwrap_or_else(|| "生成失败".into());
                return Err(ApiError(StatusCode::BAD_GATEWAY, msg));
            }
            RunStatus::Running => {
                if std::time::Instant::now() > deadline {
                    return Err(ApiError(
                        StatusCode::GATEWAY_TIMEOUT,
                        "生成超时（300s）".into(),
                    ));
                }
            }
        }
    }
}
