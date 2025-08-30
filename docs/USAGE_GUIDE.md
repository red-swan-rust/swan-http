# Swan HTTP 使用指南

## 安装

在你的 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
swan-macro = "0.1.0"
swan-common = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

## 基础用法

### 1. 定义数据结构

首先定义你的请求和响应数据结构：

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
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
```

### 2. 创建 HTTP 客户端

使用 `#[http_client]` 宏定义客户端：

```rust
use swan_macro::http_client;

#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;
```

### 3. 定义 HTTP 方法

在客户端的 impl 块中定义 HTTP 方法：

```rust
use swan_macro::{get, post, put, delete};

impl ApiClient {
    // GET 请求
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    // POST 请求
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}

    // PUT 请求
    #[put(url = "/users/1", content_type = json)]
    async fn update_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}

    // DELETE 请求
    #[delete(url = "/users/1")]
    async fn delete_user(&self) -> anyhow::Result<()> {}
}
```

### 4. 使用客户端

```rust
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
    let created = client.create_user(new_user).await?;
    println!("创建成功: {:?}", created);
    
    Ok(())
}
```

## 高级功能

### 自定义头部

```rust
impl ApiClient {
    #[get(
        url = "/protected",
        header = "Authorization: Bearer token",
        header = "X-Custom-Header: value"
    )]
    async fn get_protected(&self) -> anyhow::Result<serde_json::Value> {}
}
```

### 不同内容类型

```rust
impl ApiClient {
    // JSON (默认)
    #[post(url = "/json", content_type = json)]
    async fn post_json(&self, body: MyData) -> anyhow::Result<Response> {}

    // 表单编码
    #[post(url = "/form", content_type = form_urlencoded)]
    async fn post_form(&self, body: MyForm) -> anyhow::Result<Response> {}

    // 多部分表单
    #[post(url = "/upload", content_type = form_multipart)]
    async fn upload_file(&self, body: FileUpload) -> anyhow::Result<Response> {}
}
```

### 拦截器使用

#### 1. 定义拦截器

```rust
use async_trait::async_trait;
use swan_common::SwanInterceptor;

#[derive(Default)]
struct AuthInterceptor {
    token: String,
}

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &Vec<u8>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)> {
        let authenticated_request = request.header(
            "Authorization",
            format!("Bearer {}", self.token)
        );
        Ok((authenticated_request, request_body.clone()))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        if response.status() == 401 {
            println!("认证失败，可能需要刷新token");
        }
        Ok(response)
    }
}
```

#### 2. 全局拦截器

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = AuthInterceptor
)]
struct SecureClient;
```

#### 3. 方法级拦截器

```rust
impl SecureClient {
    #[get(url = "/data", interceptor = LoggingInterceptor)]
    async fn get_with_logging(&self) -> anyhow::Result<Data> {}
}
```

## 错误处理

Swan HTTP 使用 `anyhow::Result<T>` 进行统一错误处理：

```rust
impl ApiClient {
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}
}

// 使用
async fn example() {
    let client = ApiClient::new();
    
    match client.get_user().await {
        Ok(user) => println!("成功: {:?}", user),
        Err(e) => eprintln!("错误: {}", e),
    }
}
```

常见错误类型：
- 网络连接错误
- HTTP 状态码错误 (4xx, 5xx)
- 序列化/反序列化错误
- 拦截器处理错误

## 响应类型处理

Swan HTTP 支持多种响应类型：

```rust
impl ApiClient {
    // JSON 对象
    #[get(url = "/user")]
    async fn get_json(&self) -> anyhow::Result<User> {}

    // 原始字符串
    #[get(url = "/text")]
    async fn get_text(&self) -> anyhow::Result<String> {}

    // 原始字节
    #[get(url = "/binary")]
    async fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {}

    // 任意 JSON 值
    #[get(url = "/dynamic")]
    async fn get_value(&self) -> anyhow::Result<serde_json::Value> {}
}
```

## 最佳实践

### 1. 结构体组织

```rust
// 推荐：按功能组织客户端
#[http_client(base_url = "https://api.example.com")]
struct UserApiClient;

#[http_client(base_url = "https://api.example.com")]
struct ProductApiClient;
```

### 2. 错误处理

```rust
// 推荐：使用 ? 操作符进行错误传播
async fn example() -> anyhow::Result<()> {
    let client = ApiClient::new();
    let user = client.get_user().await?;
    let products = client.get_products().await?;
    // ...
    Ok(())
}
```

### 3. 拦截器设计

```rust
// 推荐：拦截器保持简单和专注
#[derive(Default)]
struct RetryInterceptor;

#[async_trait]
impl SwanInterceptor for RetryInterceptor {
    // 只处理重试逻辑，不做其他事情
}
```

## 调试

启用日志来调试请求和响应：

```rust
// 在 main 函数开始时
env_logger::init();

// 或者使用自定义日志级别
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
```

这将显示详细的请求和响应信息。