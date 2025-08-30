use serde::Deserialize;
use swan_macro::{http_client, get};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;

/// 用户数据结构
#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 认证拦截器 - 自动添加认证头部
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        println!("🔐 AuthInterceptor: 添加认证头部");
        let modified_request = request.header("Authorization", "Bearer demo-token-12345");
        Ok((modified_request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("🔐 AuthInterceptor: 响应状态 {}", response.status());
        Ok(response)
    }
}

/// 日志拦截器 - 记录请求和响应信息
#[derive(Default)]
struct LoggingInterceptor;

#[async_trait]
impl SwanInterceptor for LoggingInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        println!("📝 LoggingInterceptor: 记录请求，请求体大小: {} 字节", request_body.len());
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("📝 LoggingInterceptor: 响应状态: {}, 内容长度: {:?}", 
                response.status(), 
                response.headers().get("content-length"));
        Ok(response)
    }
}

/// 带全局认证拦截器的 API 客户端
#[http_client(base_url = "https://jsonplaceholder.typicode.com", interceptor = AuthInterceptor)]
struct AuthApiClient;

impl AuthApiClient {
    /// 获取用户信息（使用全局认证拦截器）
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 获取用户信息（使用方法级日志拦截器）
    #[get(url = "/users/2", interceptor = LoggingInterceptor)]
    async fn get_user_with_logging(&self) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Swan HTTP Client Interceptor Usage Example ===\n");

    let client = AuthApiClient::new();

    // 示例1：使用全局拦截器
    println!("1. 使用全局认证拦截器获取用户...");
    match client.get_user().await {
        Ok(user) => println!("   ✅ 成功获取用户: {}\n", user.name),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }

    // 示例2：使用方法级拦截器（同时也会使用全局拦截器）
    println!("2. 使用方法级日志拦截器（叠加全局认证拦截器）...");
    match client.get_user_with_logging().await {
        Ok(user) => println!("   ✅ 成功获取用户: {}\n", user.name),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }

    println!("拦截器示例运行完成！");
    
    Ok(())
}