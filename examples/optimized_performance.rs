use serde::Deserialize;
use swan_macro::{http_client, get};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use log::{info, warn, error, debug};

#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 零开销拦截器
#[derive(Default)]
struct ZeroCostInterceptor;

#[async_trait]
impl SwanInterceptor<()> for ZeroCostInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // 零开销：直接传递，无额外分配
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        // 零开销：直接传递
        Ok(response)
    }
}

/// 优化后的客户端
#[http_client(base_url = "https://jsonplaceholder.typicode.com", interceptor = ZeroCostInterceptor)]
struct OptimizedClient;

impl OptimizedClient {
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    #[get(url = "/users/{user_id}")]
    async fn get_user_dynamic(&self, user_id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Swan HTTP 优化性能演示 ===\n");

    let client = OptimizedClient::new();
    
    println!("1. 测试零开销拦截器...");
    match client.get_user().await {
        Ok(user) => info!("   ✅ 成功: {}", user.name),
        Err(e) => error!("   ❌ 错误: {}", e),
    }
    
    println!("\n2. 测试动态参数...");
    match client.get_user_dynamic(2).await {
        Ok(user) => info!("   ✅ 成功: {}", user.name),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    println!("\n🎯 性能优化特性:");
    println!("   ✅ 零拷贝：使用 Cow::Borrowed");
    println!("   ✅ 拦截器缓存：避免重复创建");
    println!("   ✅ 编译时优化：静态分发");
    
    Ok(())
}