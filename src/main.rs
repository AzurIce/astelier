mod adapter;
mod api;
mod model;
mod profiles;
mod store;
mod util;

use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

/// 前端构建产物（cd web && bun run build → web/dist）。
const WEB_DIST: &str = "web/dist";

/// 跨域：前端可作为纯静态站点部署在其他源上，把本服务选为「远端工作区」。
/// 默认放开全部来源（单用户本地工具、无鉴权，CORS 不构成额外暴露）；
/// 暴露公网时用 ASTELIER_CORS_ORIGINS="https://a,https://b" 收紧。
fn cors_layer() -> CorsLayer {
    let Ok(list) = std::env::var("ASTELIER_CORS_ORIGINS") else {
        return CorsLayer::permissive();
    };
    let origins = list
        .split(',')
        .filter_map(|s| s.trim().parse().ok())
        .collect::<Vec<_>>();
    if origins.is_empty() {
        return CorsLayer::permissive();
    }
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any)
}

#[tokio::main]
async fn main() {
    let addr = std::env::var("ASTELIER_ADDR").unwrap_or_else(|_| "127.0.0.1:8230".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("端口被占用");

    if !std::path::Path::new(WEB_DIST).join("index.html").exists() {
        println!("提示：{WEB_DIST} 不存在，仅 API 模式。构建前端：cd web && bun install && bun run build");
    }

    // SPA：静态文件优先，其余路径回退 index.html（前端路由）
    let spa =
        ServeDir::new(WEB_DIST).not_found_service(ServeFile::new(format!("{WEB_DIST}/index.html")));

    let app = axum::Router::new()
        .route("/asset/{name}", get(store::serve_asset))
        .route("/gstore/{gid}/{name}", get(store::serve_graph_store_file))
        .route("/store/{*path}", get(store::serve_global_store_file))
        .nest("/api", api::router())
        // 先注册完路由再套 body limit（layer 只作用于此前注册的路由）
        .fallback_service(spa)
        .layer(axum::extract::DefaultBodyLimit::max(128 * 1024 * 1024))
        .layer(cors_layer());

    println!("astelier → http://{addr}（API /api · 资产 /asset · 前端 {WEB_DIST}）");
    axum::serve(listener, app).await.unwrap();
}
