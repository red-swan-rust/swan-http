use serde::Deserialize;
use swan_macro::{http_client, get};
// SwanInterceptor 会由宏自动导出
use async_trait::async_trait;
use std::borrow::Cow;
use log::{info, debug, error};

#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 简单的认证拦截器 - 不写泛型，使用默认的()
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        debug!("🔐 AuthInterceptor: 添加认证头部");
        let modified_request = request.header("Authorization", "Bearer demo-token-12345");
        Ok((modified_request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        debug!("🔐 AuthInterceptor: 响应状态 {}", response.status());
        Ok(response)
    }
}

/// 日志拦截器 - 也不写泛型
#[derive(Default)]
struct LoggingInterceptor;

#[async_trait]
impl SwanInterceptor for LoggingInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        info!("📝 LoggingInterceptor: 记录请求，请求体大小: {} 字节", request_body.len());
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        info!("📝 LoggingInterceptor: 响应状态: {}", response.status());
        Ok(response)
    }
}

/// API客户端 - 使用无泛型的拦截器
#[http_client(base_url = "https://jsonplaceholder.typicode.com", interceptor = AuthInterceptor)]
struct ApiClient;

impl ApiClient {
    /// 获取用户（使用全局认证拦截器）
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 获取用户（使用方法级日志拦截器，会与全局拦截器叠加）
    #[get(url = "/users/2", interceptor = LoggingInterceptor)]
    async fn get_user_with_logging(&self) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("=== Swan HTTP 无泛型拦截器示例 ===\n");
    println!("💡 演示功能：");
    println!("   - SwanInterceptor 不写泛型（无状态拦截器）");
    println!("   - 类型安全的无状态拦截器");
    println!("   - 使用log库进行日志输出\n");

    let client = ApiClient::new();

    // 示例1：使用全局拦截器
    println!("1. 使用全局认证拦截器获取用户...");
    match client.get_user().await {
        Ok(user) => info!("✅ 成功获取用户: {}", user.name),
        Err(e) => error!("❌ 错误: {}", e),
    }

    // 示例2：使用方法级拦截器（同时也会使用全局拦截器）
    println!("\n2. 使用方法级日志拦截器（叠加全局认证拦截器）...");
    match client.get_user_with_logging().await {
        Ok(user) => info!("✅ 成功获取用户: {}", user.name),
        Err(e) => error!("❌ 错误: {}", e),
    }

    println!("\n🎯 关键说明：");
    println!("✅ 无状态拦截器使用 SwanInterceptor");
    println!("✅ 有状态拦截器使用 SwanStatefulInterceptor<State>");
    println!("✅ IDE只会提示对应的trait，避免混淆");
    
    Ok(())
}