# Swan HTTP API 文档

## 过程宏

### `#[http_client]`

用于定义 HTTP 客户端结构体的宏。

#### 语法

```rust
#[http_client(base_url = "URL", interceptor = InterceptorType)]
struct ClientName;
```

#### 参数

- `base_url` (可选): 客户端的基础 URL
- `interceptor` (可选): 全局拦截器类型

#### 示例

```rust
#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;
```

### HTTP 方法宏

#### `#[get]`

定义 GET 请求方法。

```rust
#[get(url = "/path", header = "Key: Value", interceptor = InterceptorType)]
async fn method_name(&self) -> anyhow::Result<ResponseType> {}
```

#### `#[post]`

定义 POST 请求方法。

```rust
#[post(url = "/path", content_type = json, header = "Key: Value")]
async fn method_name(&self, body: RequestType) -> anyhow::Result<ResponseType> {}
```

#### `#[put]`

定义 PUT 请求方法。

#### `#[delete]`

定义 DELETE 请求方法。

#### 参数

- `url` (必需): 请求的相对 URL
- `content_type` (可选): 内容类型 (`json`, `form_urlencoded`, `form_multipart`)
- `header` (可选): 自定义头部，格式为 "Key: Value"
- `interceptor` (可选): 方法级拦截器

## 核心类型

### `HttpMethod`

HTTP 方法枚举。

```rust
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}
```

#### 方法

- `as_str() -> &'static str`: 返回 HTTP 方法字符串
- `client_method() -> Ident`: 返回客户端方法标识符

### `ContentType`

内容类型枚举。

```rust
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormMultipart,
}
```

### `SwanInterceptor`

拦截器 trait，用于在请求前后进行自定义处理。

```rust
#[async_trait]
pub trait SwanInterceptor {
    async fn before_request(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &Vec<u8>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)>;

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response>;
}
```

#### 方法

- `before_request`: 在请求发送前调用，可以修改请求和请求体
- `after_response`: 在响应接收后调用，可以修改响应

## 使用模式

### 1. 简单 HTTP 客户端

```rust
#[http_client(base_url = "https://api.example.com")]
struct SimpleClient;

impl SimpleClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
}
```

### 2. 带认证的客户端

```rust
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request(&self, request: reqwest::RequestBuilder, body: &Vec<u8>) 
        -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)> {
        let authenticated_request = request.header("Authorization", "Bearer token");
        Ok((authenticated_request, body.clone()))
    }
    
    async fn after_response(&self, response: reqwest::Response) 
        -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct AuthClient;
```

### 3. 复合拦截器

```rust
impl AuthClient {
    // 同时使用全局认证拦截器和方法级日志拦截器
    #[get(url = "/protected", interceptor = LoggingInterceptor)]
    async fn get_protected_data(&self) -> anyhow::Result<SecretData> {}
}
```

## 错误处理

所有生成的方法都返回 `anyhow::Result<T>`，提供统一的错误处理：

- 网络错误
- 序列化/反序列化错误
- HTTP 状态码错误
- 拦截器错误

## 类型转换

Swan HTTP 支持多种响应类型的自动转换：

- `String`: 直接转换为 UTF-8 字符串
- `Vec<u8>`: 返回原始字节数组
- 自定义类型: 通过 serde_json 进行 JSON 反序列化

## 日志

库内置了请求和响应的日志记录，使用 `log` crate。要启用日志：

```rust
env_logger::init();
```