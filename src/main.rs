mod adapter;
mod api;
mod model;
mod profiles;
mod store;
mod util;

use axum::routing::get;
use tower_http::services::{ServeDir, ServeFile};

/// 前端构建产物（cd web && bun run build → web/dist）。
const WEB_DIST: &str = "web/dist";

#[tokio::main]
async fn main() {
    let addr = std::env::var("ATLIER_ADDR").unwrap_or_else(|_| "127.0.0.1:8230".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("端口被占用");

    if !std::path::Path::new(WEB_DIST).join("index.html").exists() {
        println!("提示：{WEB_DIST} 不存在，仅 API 模式。构建前端：cd web && bun install && bun run build");
    }

    // SPA：静态文件优先，其余路径回退 index.html（前端路由）
    let spa = ServeDir::new(WEB_DIST).not_found_service(ServeFile::new(format!("{WEB_DIST}/index.html")));

    let app = axum::Router::new()
        .route("/asset/{name}", get(store::serve_asset))
        .route("/gstore", get(store::serve_store_file))
        .nest("/api", api::router())
        // 先注册完路由再套 body limit（layer 只作用于此前注册的路由）
        .fallback_service(spa)
        .layer(axum::extract::DefaultBodyLimit::max(128 * 1024 * 1024));

    println!("atelier → http://{addr}（API /api · 资产 /asset · 前端 {WEB_DIST}）");
    axum::serve(listener, app).await.unwrap();
}
