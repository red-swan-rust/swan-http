use serde::Deserialize;
use swan_macro::{http_client, get};

#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 简单重试测试客户端
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct SimpleRetryClient;

impl SimpleRetryClient {
    /// 无重试的基础方法
    #[get(url = "/users/1")]
    async fn get_user_no_retry(&self) -> anyhow::Result<User> {}
    
    /// 带重试的方法
    #[get(url = "/users/1", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== 简单重试测试 ===");
    
    let client = SimpleRetryClient::new();
    
    println!("测试无重试...");
    match client.get_user_no_retry().await {
        Ok(user) => println!("✅ 成功: {:?}", user.name),
        Err(e) => println!("❌ 失败: {}", e),
    }
    
    println!("\n测试带重试...");
    match client.get_user_with_retry().await {
        Ok(user) => println!("✅ 成功: {:?}", user.name),
        Err(e) => println!("❌ 失败: {}", e),
    }
    
    Ok(())
}