use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post};

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

/// HTTP 代理客户端示例
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    proxy = "http://proxy.example.com:8080"
)]
struct HttpProxyClient;

/// SOCKS5 代理客户端示例 (注意：需要reqwest开启socks feature)
// #[http_client(
//     base_url = "https://api.example.com",
//     proxy = "socks5://socks-proxy.example.com:1080"
// )]
// struct Socks5ProxyClient;

/// 带认证的代理客户端示例
#[http_client(
    base_url = "https://secure-api.example.com",
    proxy(url = "auth-proxy.example.com:3128", username = "proxyuser", password = "proxypass")
)]
struct AuthProxyClient;

/// 禁用代理的客户端示例
#[http_client(
    base_url = "https://local-api.example.com",
    proxy = false
)]
struct NoProxyClient;

/// 混合代理使用示例 - 客户端有默认代理，但方法可以覆盖
#[http_client(
    base_url = "https://mixed-api.example.com",
    proxy = "http://default-proxy.example.com:8080"
)]
struct MixedClient;

impl HttpProxyClient {
    /// 使用客户端默认的 HTTP 代理
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 创建用户（使用客户端默认代理）
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, _body: CreateUserRequest) -> anyhow::Result<User> {}
}

// impl Socks5ProxyClient {
//     /// 使用客户端默认的 SOCKS5 代理
//     #[get(url = "/secure-data")]
//     async fn get_secure_data(&self) -> anyhow::Result<Vec<User>> {}
// }

impl AuthProxyClient {
    /// 使用需要认证的代理
    #[get(url = "/authenticated-endpoint")]
    async fn get_authenticated(&self) -> anyhow::Result<User> {}
}

impl NoProxyClient {
    /// 不使用任何代理（直接连接）
    #[get(url = "/direct")]
    async fn get_direct(&self) -> anyhow::Result<User> {}
}

impl MixedClient {
    /// 使用客户端默认的 HTTP 代理
    #[get(url = "/default")]
    async fn with_default_proxy(&self) -> anyhow::Result<User> {}

    /// 方法级别覆盖：使用 SOCKS5 代理
    #[get(url = "/socks", proxy = "socks5://method-socks.example.com:1080")]
    async fn with_socks_proxy(&self) -> anyhow::Result<User> {}

    /// 方法级别覆盖：禁用代理
    #[get(url = "/direct", proxy = false)]
    async fn without_proxy(&self) -> anyhow::Result<User> {}

    /// 方法级别覆盖：使用不同的 HTTP 代理
    #[post(url = "/special", content_type = json, proxy = "http://special-proxy.example.com:9090")]
    async fn with_special_proxy(&self, _body: CreateUserRequest) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== Swan HTTP Client Proxy Usage Examples ===\n");

    // 注意：这些示例需要相应的代理服务器才能正常工作
    // 在实际使用中，请替换为您的真实代理服务器地址

    // HTTP 代理客户端
    let http_client = HttpProxyClient::new();
    println!("✅ HTTP 代理客户端已创建");

    // SOCKS5 代理客户端 (需要reqwest开启socks feature)
    // let socks5_client = Socks5ProxyClient::new();
    // println!("✅ SOCKS5 代理客户端已创建");

    // 带认证的代理客户端
    let auth_client = AuthProxyClient::new();
    println!("✅ 认证代理客户端已创建");

    // 无代理客户端
    let no_proxy_client = NoProxyClient::new();
    println!("✅ 无代理客户端已创建");

    // 混合代理客户端
    let mixed_client = MixedClient::new();
    println!("✅ 混合代理客户端已创建");

    // 示例请求（注释掉以避免实际网络请求）
    /*
    match http_client.get_user().await {
        Ok(user) => println!("✅ HTTP 代理请求成功: {}", user.name),
        Err(e) => println!("❌ HTTP 代理请求失败: {}", e),
    }

    match mixed_client.without_proxy().await {
        Ok(user) => println!("✅ 无代理请求成功: {}", user.name),
        Err(e) => println!("❌ 无代理请求失败: {}", e),
    }
    */

    println!("\n🎉 代理配置示例完成！");
    println!("\n📝 代理配置选项：");
    println!("  • HTTP/HTTPS: proxy = \"http://proxy.com:8080\"");
    println!("  • SOCKS5: proxy = \"socks5://proxy.com:1080\"");
    println!("  • 显式类型: proxy(type = http, url = \"proxy.com:8080\")");
    println!("  • 带认证: proxy(type = socks5, url = \"proxy.com:1080\", username = \"user\", password = \"pass\")");
    println!("  • 禁用代理: proxy = false");
    println!("  • 方法级覆盖: #[get(url = \"/path\", proxy = \"...\")]");

    Ok(())
}