use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use log::{info, warn, error, debug};

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

/// 简单的认证拦截器（无状态）
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
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let request = request.header("Authorization", format!("Bearer {}", self.token));
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        info!("请求完成，状态码: {}", response.status());
        Ok(response)
    }
}

/// API 客户端（无状态）
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== Swan HTTP Client Basic Usage Example ===\n");

    let client = ApiClient::new();
    
    match client.get_user().await {
        Ok(user) => info!("✅ 获取用户: {}\n", user.name),
        Err(e) => error!("❌ 错误: {}\n", e),
    }

    let new_user = CreateUserRequest {
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
    };
    
    match client.create_user(new_user).await {
        Ok(user) => info!("✅ 创建的用户: {}\n", user.name),
        Err(e) => error!("❌ 错误: {}\n", e),
    }

    println!("🎉 基础示例运行完成！");
    
    Ok(())
}