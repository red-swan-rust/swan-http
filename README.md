# Swan HTTP - 声明式 Rust HTTP 客户端

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

🌏 **Languages**: [English](README_EN.md) | [中文](README.md)

Swan HTTP 是一个现代的、声明式的 Rust HTTP 客户端库，通过过程宏提供优雅的 API 设计。

## 📦 Crates

Swan HTTP 由两个独立的 crate 组成：

- **[swan-macro](https://crates.io/crates/swan-macro)** [![Crates.io](https://img.shields.io/crates/v/swan-macro.svg)](https://crates.io/crates/swan-macro) - 过程宏组件
- **[swan-common](https://crates.io/crates/swan-common)** [![Crates.io](https://img.shields.io/crates/v/swan-common.svg)](https://crates.io/crates/swan-common) - 核心运行时组件

## 🌟 特性

- **声明式设计**: 使用宏注解定义 HTTP 客户端和方法
- **类型安全**: 完全的 Rust 类型安全，编译时错误检查
- **拦截器支持**: 灵活的全局和方法级拦截器系统
- **🆕 状态注入**: 类似 Axum 的应用状态管理，支持依赖注入
- **🆕 动态参数**: URL和header中的参数占位符，支持 `{param_name}` 和 `{param0}` 语法
- **🔄 智能重试**: 方法级渐进式指数退避重试，支持幂等性保护和智能重试条件
- **多种内容类型**: 支持 JSON、表单和多部分表单数据
- **异步优先**: 基于 tokio 的异步设计
- **高性能优化**: 零拷贝、拦截器缓存、条件编译优化
- **模块化架构**: 清晰的模块分离，易于维护和扩展

## 🚀 快速开始

将以下内容添加到你的 `Cargo.toml`:

```toml
[dependencies]
swan-macro = "0.2"   # 过程宏组件
swan-common = "0.2"  # 核心运行时组件
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

> **注意**: 需要同时添加 `swan-macro` 和 `swan-common` 两个依赖才能正常使用 Swan HTTP。

### 基本用法

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
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;

impl ApiClient {
    // GET 请求
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    // POST 请求
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    // 带重试的 GET 请求
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self, id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ApiClient::new();
    
    // 获取用户
    let user = client.get_user().await?;
    println!("用户: {:?}", user);
    
    // 创建用户
    let new_user = CreateUserRequest {
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
    };
    let created_user = client.create_user(new_user).await?;
    println!("创建的用户: {:?}", created_user);
    
    Ok(())
}
```

## 🔧 高级功能

### 🔄 重试机制

Swan HTTP 提供强大的方法级重试功能，支持智能的指数退避算法：

```rust
impl ApiClient {
    // 基础指数重试：最多3次，基础延迟100ms
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    // 详细配置：自定义所有参数
    #[get(url = "/external/api", retry = "exponential(
        max_attempts=5,
        base_delay=200ms,
        max_delay=30s,
        exponential_base=2.0,
        jitter_ratio=0.1,
        idempotent_only=true
    )")]
    async fn call_external_api(&self) -> anyhow::Result<Data> {}
    
    // 固定延迟重试：适用于稳定服务
    #[get(url = "/stable/service", retry = "fixed(max_attempts=4, delay=500ms)")]
    async fn call_stable_service(&self) -> anyhow::Result<Data> {}
}
```

**重试特性：**
- **智能重试条件**: 自动重试 5xx 错误、429 限流、408 超时
- **幂等性保护**: 默认只重试安全的 GET/PUT/DELETE 方法
- **指数退避**: 避免服务器过载，支持自定义增长速度
- **随机抖动**: 防止雷群效应，分散重试时间
- **灵活配置**: 支持简化和详细两种配置语法

详细的重试机制文档请参考: [docs/retry_mechanism.md](docs/retry_mechanism.md)

### 拦截器

拦截器允许你在请求发送前和响应接收后进行自定义处理：

```rust
use async_trait::async_trait;
use swan_common::SwanInterceptor;
use std::borrow::Cow;
use std::any::Any;

#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>, // 👈 状态上下文
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let modified_request = request.header("Authorization", "Bearer token");
        // 零拷贝优化：直接借用请求体，避免克隆
        Ok((modified_request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>, // 👈 状态上下文
    ) -> anyhow::Result<reqwest::Response> {
        println!("响应状态: {}", response.status());
        Ok(response)
    }
}

// 使用全局拦截器
#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;

impl SecureApiClient {
    // 使用方法级拦截器（会与全局拦截器叠加）
    #[get(url = "/protected", interceptor = LoggingInterceptor)]
    async fn get_protected_data(&self) -> anyhow::Result<serde_json::Value> {}
}
```

### 🆕 状态注入

Swan HTTP 支持类似 Axum 的应用状态管理，适用于依赖注入场景：

```rust
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// 1. 定义应用状态
#[derive(Clone)]
struct AppState {
    cache: Arc<RwLock<HashMap<String, String>>>,
    request_counter: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut cache = HashMap::new();
        cache.insert("auth_token".to_string(), "cached-jwt-token".to_string());
        
        Self {
            cache: Arc::new(RwLock::new(cache)),
            request_counter: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn get_cached_token(&self) -> Option<String> {
        self.cache.read().unwrap().get("auth_token").cloned()
    }
}

// 2. 创建状态感知的拦截器
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
        
        // 从context获取状态
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                if let Some(token) = app_state.get_cached_token().await {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
            }
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

// 3. 声明状态类型
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState  // 👈 声明状态类型
)]
struct StatefulApiClient;

impl StatefulApiClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
}

// 4. 使用链式调用注入状态
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = AppState::new();
    
    let client = StatefulApiClient::new()
        .with_state(app_state); // 👈 注入状态
    
    let users = client.get_users().await?;
    Ok(())
}
```

详细的状态注入文档请参考: [docs/STATE_INJECTION.md](docs/STATE_INJECTION.md)

### 支持的 HTTP 方法

- `#[get]` - GET 请求
- `#[post]` - POST 请求  
- `#[put]` - PUT 请求
- `#[delete]` - DELETE 请求

### 内容类型

- `json` - application/json
- `form_urlencoded` - application/x-www-form-urlencoded
- `form_multipart` - multipart/form-data

### 🆕 动态参数

支持在URL和header中使用动态参数占位符，运行时自动替换：

```rust
impl ApiClient {
    // URL路径参数
    #[get(url = "/users/{user_id}/posts/{post_id}")]
    async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}
    
    // 查询参数
    #[get(url = "/search?q={query}&page={page}")]
    async fn search(&self, query: String, page: u32) -> anyhow::Result<Vec<Post>> {}
    
    // Header动态值
    #[post(
        url = "/users/{user_id}/posts",
        content_type = json,
        header = "Authorization: Bearer {auth_token}",
        header = "X-User-ID: {user_id}"
    )]
    async fn create_post(&self, user_id: u32, auth_token: String, body: CreatePostRequest) -> anyhow::Result<Post> {}
    
    // 按位置引用参数（param0, param1, ...）
    #[get(
        url = "/posts?author={param0}&category={param1}",
        header = "X-Author: {param0}",
        header = "X-Category: {param1}"
    )]
    async fn search_by_position(&self, author: String, category: String) -> anyhow::Result<Vec<Post>> {}
}
```

**占位符语法：**
- `{param_name}` - 按参数名称引用
- `{param0}`, `{param1}` - 按参数位置引用（从0开始，跳过self参数）

### 自定义头部

```rust
impl ApiClient {
    #[get(
        url = "/api/data",
        header = "Authorization: Bearer token",
        header = "X-Custom-Header: custom-value"
    )]
    async fn get_with_headers(&self) -> anyhow::Result<serde_json::Value> {}
}
```

## 📁 项目架构

重构后的项目采用清晰的模块化架构：

```
swan-http/
├── swan-common/          # 核心类型和工具
│   ├── types/           # HTTP 方法、内容类型等
│   ├── parsing/         # 宏参数解析逻辑  
│   └── interceptor/     # 拦截器 trait 定义
├── swan-macro/          # 过程宏实现
│   ├── generator/       # 代码生成逻辑
│   ├── conversion/      # 类型转换逻辑
│   ├── request/         # 请求处理逻辑
│   └── error/           # 错误处理逻辑
├── tests/               # 集成测试
└── examples/            # 使用示例
```

这种模块化设计解决了原有代码"牵一发动全身"的问题，每个模块都有明确的职责边界。

## 🔍 重构改进

相比重构前的版本，新架构具有以下优势：

1. **职责分离**: 每个模块负责特定功能，降低耦合度
2. **易于维护**: 修改一个功能不会影响其他不相关功能
3. **易于测试**: 每个模块都可以独立测试
4. **易于扩展**: 新功能可以独立添加到相应模块
5. **文档完善**: 每个模块和函数都有完整的文档

## ⚡ 性能优化

Swan HTTP 库实现了多项性能优化技术：

### 1. 拦截器对象池化/缓存
- 使用 `InterceptorCache` 避免重复创建拦截器实例
- 采用 `Arc<T>` 共享拦截器，降低内存分配开销
- 客户端级别缓存，避免 Box 装箱成本

### 2. 零拷贝优化
- 统一的 `SwanInterceptor` trait 使用 `Cow<[u8]>` 避免不必要的内存拷贝
- 只有在真正需要修改请求体时才进行数据克隆
- 大幅降低内存分配和拷贝开销

### 3. 延迟序列化优化
- 智能检测是否需要序列化请求体
- GET/DELETE 请求跳过序列化过程
- 只在有拦截器需要访问请求体时才进行序列化

### 4. 条件编译优化
- 使用 `#[cfg(debug_assertions)]` 在 release 模式下完全移除调试代码
- 编译时优化，零运行时开销
- 显著减少 release 版本的代码体积

这些优化确保了 Swan HTTP 在保持功能完整性的同时，具备出色的性能表现。

### 性能优化使用建议

1. **拦截器设计**：实现拦截器时优先使用 `Cow::Borrowed(request_body)` 避免不必要的克隆
2. **依赖管理**：在你的项目中添加 `env_logger` 或其他日志实现来启用调试日志
3. **发布构建**：使用 `cargo build --release` 来获得最佳性能，调试代码会被完全移除
4. **复杂API**：参考 `complex_api_example.rs` 了解如何处理企业级API的复杂认证和头部

## 🧪 运行测试

```bash
# 运行所有单元测试
cargo test --lib

# 运行集成测试
cargo test --test integration_test

# 运行示例
cargo run --example basic_usage           # 基础用法示例（包含状态注入）
cargo run --example interceptor_usage     # 拦截器用法示例  
cargo run --example dynamic_params_example # 🆕 动态参数示例（URL和header占位符）
cargo run --example complex_api_example   # 企业级API示例（性能优化+状态管理）
cargo run --example state_injection_example # 🆕 状态注入完整示例
cargo run --example simple_retry_test     # 🔄 简单重试功能测试
cargo run --example retry_integration_test # 🔄 重试机制集成测试
```

## 📖 API 文档

### 在线文档

- **[swan-macro 文档](https://docs.rs/swan-macro)** - 过程宏 API 文档
- **[swan-common 文档](https://docs.rs/swan-common)** - 核心类型和拦截器 API 文档

### 本地文档

详细的 API 文档可以通过以下命令生成并查看：

```bash
# 生成所有组件的文档
cargo doc --open

# 或者生成特定组件的文档
cargo doc --open -p swan-macro
cargo doc --open -p swan-common
```

## 🤝 贡献

欢迎贡献！请随时提交 issue 或 pull request。

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](LICENSE) 文件。