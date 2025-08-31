// 此示例展示统一的 SwanInterceptor 接口
// 使用泛型参数来区分有状态和无状态的拦截器

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

struct AppState {
    tenant_id: String,
}

/// 正确示例: 无状态客户端使用无状态拦截器
#[derive(Default)]
struct StatelessInterceptor;

#[async_trait]
impl SwanInterceptor<()> for StatelessInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        info!("✅ 无状态拦截器");
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

/// 有状态拦截器
#[derive(Default)]
struct StatefulInterceptor;

#[async_trait]
impl SwanInterceptor<AppState> for StatefulInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        state: Option<&AppState>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        if let Some(app_state) = state {
            info!("✅ 有状态拦截器: {}", app_state.tenant_id);
        }
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&AppState>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[http_client(base_url = "https://jsonplaceholder.typicode.com", interceptor = StatelessInterceptor)]
struct StatelessClient;

impl StatelessClient {
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}
}

#[http_client(base_url = "https://jsonplaceholder.typicode.com", interceptor = StatefulInterceptor, state = AppState)]
struct StatefulClient;

impl StatefulClient {
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 统一 SwanInterceptor 接口演示 ===\n");
    
    println!("✅ 无状态客户端使用 SwanInterceptor<()>");
    let client = StatelessClient::new();
    match client.get_user().await {
        Ok(user) => info!("   成功: {}", user.name),
        Err(e) => error!("   错误: {}", e),
    }
    
    println!("\n✅ 有状态客户端使用 SwanInterceptor<AppState>");
    let state = AppState { tenant_id: "tenant-123".to_string() };
    let stateful_client = StatefulClient::new().with_state(state);
    match stateful_client.get_user().await {
        Ok(user) => info!("   成功: {}", user.name),
        Err(e) => error!("   错误: {}", e),
    }
    
    println!("\n💡 统一接口的优势：");
    println!("   - 同一个 SwanInterceptor trait");
    println!("   - 用泛型参数 <State> 区分有状态/无状态");
    println!("   - 类型安全，无需 downcast");
    
    Ok(())
}