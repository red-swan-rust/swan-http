# Swan Macro

[![Crates.io](https://img.shields.io/crates/v/swan-macro.svg)](https://crates.io/crates/swan-macro)
[![Documentation](https://docs.rs/swan-macro/badge.svg)](https://docs.rs/swan-macro)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

🌏 **Languages**: [English](README.md) | [中文](README_CN.md)

Swan Macro 是 Swan HTTP 库的过程宏组件，提供声明式的 HTTP 客户端定义语法。

## 🌟 核心功能

- **声明式客户端定义**: 使用宏注解定义 HTTP 客户端和方法
- **自动代码生成**: 编译时生成高性能的 HTTP 客户端代码
- **智能重试机制**: 方法级渐进式指数退避重试
- **拦截器集成**: 无缝集成全局和方法级拦截器
- **动态参数支持**: URL 和 header 中的参数占位符
- **状态注入**: 类似 Axum 的应用状态管理

## 📦 安装

将以下内容添加到你的 `Cargo.toml`:

```toml
[dependencies]
swan-macro = "0.2"
swan-common = "0.2"  # 必需的运行时依赖
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

## 🚀 快速开始

### 基础用法

```rust
use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post};

#[derive(Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Serialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

// 定义 HTTP 客户端
#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    // GET 请求
    #[get(url = "/users/{id}")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}

    // POST 请求
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    // 带重试的请求
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self, id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ApiClient::new();
    
    // 使用客户端
    let user = client.get_user(1).await?;
    println!("用户: {}", user.name);
    
    Ok(())
}
```

## 🔧 支持的宏

### `#[http_client]`

定义 HTTP 客户端结构体：

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = MyInterceptor,  // 可选：全局拦截器
    state = AppState              // 可选：应用状态类型
)]
struct ApiClient;
```

### HTTP 方法宏

支持的 HTTP 方法：

- `#[get(url = "...")]` - GET 请求
- `#[post(url = "...", content_type = json)]` - POST 请求
- `#[put(url = "...", content_type = json)]` - PUT 请求  
- `#[delete(url = "...")]` - DELETE 请求

### 方法参数

```rust
impl ApiClient {
    #[get(
        url = "/users/{id}",                    // 路径参数
        header = "Authorization: Bearer {token}", // 动态头部
        retry = "exponential(3, 100ms)",        // 重试策略
        interceptor = MethodLevelInterceptor    // 方法级拦截器
    )]
    async fn get_user(&self, id: u32, token: String) -> anyhow::Result<User> {}
}
```

## 🔄 重试机制

### 重试策略类型

```rust
// 指数退避重试
#[get(url = "/api", retry = "exponential(3, 100ms)")]
#[get(url = "/api", retry = "exponential(
    max_attempts=5,
    base_delay=200ms,
    max_delay=30s,
    exponential_base=2.0,
    jitter_ratio=0.1
)")]

// 固定延迟重试
#[get(url = "/api", retry = "fixed(max_attempts=3, delay=1s)")]
```

### 自动重试条件

- **5xx 服务器错误** (500-599)
- **429 Too Many Requests** (限流)
- **408 Request Timeout** (超时)
- **网络连接错误**

### 幂等性保护

默认只对安全的HTTP方法重试：

```rust
#[get(url = "/data")]     // ✅ 自动重试
#[put(url = "/data")]     // ✅ 自动重试  
#[delete(url = "/data")]  // ✅ 自动重试
#[post(url = "/data")]    // ❌ 默认不重试 (非幂等)

// 强制重试非幂等方法 (谨慎使用)
#[post(url = "/idempotent", retry = "exponential(
    max_attempts=3,
    base_delay=100ms,
    idempotent_only=false
)")]
```

## 🌐 动态参数

### URL 参数

```rust
// 路径参数
#[get(url = "/users/{user_id}/posts/{post_id}")]
async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}

// 查询参数
#[get(url = "/search?q={query}&page={page}")]
async fn search(&self, query: String, page: u32) -> anyhow::Result<Vec<Post>> {}

// 按位置引用参数
#[get(url = "/posts?author={param0}&category={param1}")]
async fn search_by_position(&self, author: String, category: String) -> anyhow::Result<Vec<Post>> {}
```

### 动态头部

```rust
#[get(
    url = "/protected",
    header = "Authorization: Bearer {token}",
    header = "X-User-ID: {user_id}"
)]
async fn get_protected_data(&self, token: String, user_id: u32) -> anyhow::Result<Data> {}
```

## 🔌 拦截器集成

```rust
use async_trait::async_trait;
use swan_common::SwanInterceptor;

#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn std::any::Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, std::borrow::Cow<'a, [u8]>)> {
        let request = request.header("Authorization", "Bearer token");
        Ok((request, std::borrow::Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn std::any::Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("响应状态: {}", response.status());
        Ok(response)
    }
}

// 使用拦截器
#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;
```

## 🏷️ 内容类型

支持的内容类型：

- `content_type = json` - application/json
- `content_type = form_urlencoded` - application/x-www-form-urlencoded
- `content_type = form_multipart` - multipart/form-data

## ⚡ 编译时优化

Swan Macro 在编译时生成高度优化的代码：

- **零运行时开销**: 所有配置在编译时确定
- **内联优化**: 自动内联小函数调用
- **条件编译**: 在 release 模式下移除调试代码
- **智能缓存**: 拦截器实例复用

## 🧪 测试

运行测试：

```bash
cargo test --lib
```

## 📖 文档

详细的 API 文档：

```bash
cargo doc --open
```

## 🤝 与 Swan Common 配合使用

Swan Macro 依赖 [Swan Common](https://crates.io/crates/swan-common) 提供运行时支持：

```toml
[dependencies]
swan-macro = "0.2"
swan-common = "0.2"
```

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](../LICENSE) 文件。