use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;

/// 用户数据结构
#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 创建用户请求
#[derive(Serialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

/// 简单的认证拦截器
#[derive(Default)]
struct AuthInterceptor {
    token: String,
}

impl AuthInterceptor {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let request = request.header("Authorization", format!("Bearer {}", self.token));
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("请求完成，状态码: {}", response.status());
        Ok(response)
    }
}

/// 简单应用状态
#[derive(Clone)]
struct AppConfig {
    api_key: String,
    user_agent: String,
}

/// 状态感知的拦截器
#[derive(Default)]
struct StateAwareInterceptor;

#[async_trait]
impl SwanInterceptor for StateAwareInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let mut request = request;
        
        // 从context获取应用配置
        if let Some(ctx) = context {
            if let Some(config) = ctx.downcast_ref::<AppConfig>() {
                request = request
                    .header("X-API-Key", &config.api_key)
                    .header("User-Agent", &config.user_agent);
                println!("🔑 使用配置中的API密钥: {}...", &config.api_key[..8]);
            }
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("✅ 请求完成，状态码: {}", response.status());
        Ok(response)
    }
}

/// API 客户端（无状态）
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;

/// 带状态的 API 客户端
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    interceptor = StateAwareInterceptor,
    state = AppConfig
)]
struct StatefulApiClient;

impl ApiClient {
    /// 获取用户信息
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 创建新用户
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}

    /// 获取所有用户
    #[get(url = "/users")]
    async fn get_all_users(&self) -> anyhow::Result<Vec<User>> {}
}

impl StatefulApiClient {
    /// 获取用户信息（使用状态感知拦截器）
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 获取所有用户（使用状态感知拦截器）
    #[get(url = "/users")]
    async fn get_all_users(&self) -> anyhow::Result<Vec<User>> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    env_logger::init();

    println!("=== Swan HTTP Client Basic Usage Example ===\n");

    // 示例1：基础客户端（无状态）
    println!("1. 基础客户端示例（无状态）");
    let client = ApiClient::new();
    
    match client.get_user().await {
        Ok(user) => println!("   ✅ 获取用户: {}\n", user.name),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }

    // 示例2：带状态的客户端
    println!("2. 状态感知客户端示例");
    
    // 创建应用配置
    let config = AppConfig {
        api_key: "secret-api-key-12345".to_string(),
        user_agent: "Swan-HTTP-Client/1.0".to_string(),
    };
    
    // 创建带状态的客户端
    let stateful_client = StatefulApiClient::new()
        .with_state(config);
    
    match stateful_client.get_user().await {
        Ok(user) => println!("   ✅ 获取用户: {}", user.name),
        Err(e) => println!("   ❌ 错误: {}", e),
    }

    match stateful_client.get_all_users().await {
        Ok(users) => println!("   ✅ 获取 {} 个用户\n", users.len()),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }

    // 示例3：创建新用户（使用基础客户端）
    println!("3. 创建新用户...");
    let new_user = CreateUserRequest {
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
    };
    
    match client.create_user(new_user).await {
        Ok(user) => println!("   ✅ 创建的用户: {}\n", user.name),
        Err(e) => println!("   ❌ 错误: {}\n", e),
    }

    println!("🎉 所有示例运行完成！");
    
    Ok(())
}