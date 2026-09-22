#![allow(non_snake_case)]

mod api;
mod app;
mod model;
mod profiles;
mod ui;
mod util;

// 服务端独占模块（wasm 客户端不编译）
#[cfg(feature = "server")]
mod adapter;
#[cfg(feature = "server")]
mod store;

fn main() {
    #[cfg(feature = "server")]
    {
        server_main();
    }
    #[cfg(not(feature = "server"))]
    {
        dioxus::launch(app::App);
    }
}

/// server 入口：自建 axum Router（挂资产路由）+ Dioxus SSR fallback。
#[cfg(feature = "server")]
fn server_main() {
    use dioxus::server::DioxusRouterExt;

    dioxus::serve(|| async {
        crate::store::migrate_legacy().await;
        let router = axum::Router::new()
            .route("/asset/{name}", axum::routing::get(store::serve_asset))
            // 注意顺序：先注册完所有路由（含 server functions），再套
            // DefaultBodyLimit —— Router::layer 只作用于此前已注册的路由
            .serve_dioxus_application(dioxus::server::ServeConfig::new(), app::App)
            .layer(axum::extract::DefaultBodyLimit::max(128 * 1024 * 1024));
        Ok(router)
    });
}

#[cfg(all(test, feature = "server"))]
mod migrate_tests {
    #[tokio::test]
    async fn minimal_runtime_works() {
        eprintln!("[minimal] runtime ok");
    }
    #[tokio::test]
    async fn migrate_completes() {
        eprintln!("[migrate] enter");
        crate::store::migrate_legacy().await;
        eprintln!("[migrate] done");
    }
}
