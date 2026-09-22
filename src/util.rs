//! 跨平台小工具：延时（wasm / native）、当前时间。

/// 异步延时：wasm 客户端走 gloo timers，server 走 tokio。
pub async fn delay(ms: u64) {
    #[cfg(all(target_arch = "wasm32", feature = "web"))]
    {
        use gloo_timers::future::TimeoutFuture;
        TimeoutFuture::new(ms as u32).await;
    }
    #[cfg(all(feature = "server", not(target_arch = "wasm32")))]
    {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
    }
    #[cfg(not(any(feature = "server", feature = "web")))]
    {
        let _ = ms; // 纯 check 编译路径，不运行
    }
}

pub fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        js_sys::Date::now() as u64
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
