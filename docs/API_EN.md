# Swan HTTP API Documentation

🌏 **Languages**: [English](API_EN.md) | [中文](API.md)

## Procedural Macros

### `#[http_client]`

A macro for defining HTTP client structs.

#### Syntax

```rust
#[http_client(base_url = "URL", interceptor = InterceptorType)]
struct ClientName;
```

#### Parameters

- `base_url` (optional): Base URL for the client
- `interceptor` (optional): Global interceptor type

#### Examples

```rust
#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;
```

### HTTP Method Macros

#### `#[get]`

Defines a GET request method.

```rust
#[get(url = "/path", header = "Key: Value", interceptor = InterceptorType)]
async fn method_name(&self) -> anyhow::Result<ResponseType> {}
```

#### `#[post]`

Defines a POST request method.

```rust
#[post(url = "/path", content_type = json, header = "Key: Value")]
async fn method_name(&self, body: RequestType) -> anyhow::Result<ResponseType> {}
```

#### `#[put]`

Defines a PUT request method.

#### `#[delete]`

Defines a DELETE request method.

#### Parameters

- `url` (required): Relative URL for the request
- `content_type` (optional): Content type (`json`, `form_urlencoded`, `form_multipart`)
- `header` (optional): Custom header in "Key: Value" format
- `interceptor` (optional): Method-level interceptor

## Core Types

### `HttpMethod`

HTTP method enumeration.

```rust
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}
```

#### Methods

- `as_str() -> &'static str`: Returns the HTTP method string
- `client_method() -> Ident`: Returns the client method identifier

### `ContentType`

Content type enumeration.

```rust
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormMultipart,
}
```

### `SwanInterceptor<State>`

Interceptor trait for custom processing before and after requests. Now supports type-safe state injection.

```rust
#[async_trait]
pub trait SwanInterceptor<State> {
    async fn before_request(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &Vec<u8>,
        state: Option<&State>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)>;

    async fn after_response(
        &self,
        response: reqwest::Response,
        state: Option<&State>,
    ) -> anyhow::Result<reqwest::Response>;
}
```

#### Methods

- `before_request`: Called before sending the request, can modify the request and request body, supports type-safe state access
- `after_response`: Called after receiving the response, can modify the response, supports type-safe state access

#### State Types

- For stateless interceptors: use `SwanInterceptor<()>`
- For stateful interceptors: use `SwanInterceptor<YourStateType>` for type-safe state access

## Usage Patterns

### 1. Simple HTTP Client

```rust
#[http_client(base_url = "https://api.example.com")]
struct SimpleClient;

impl SimpleClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
}
```

### 2. Authenticated Client

```rust
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor<()> for AuthInterceptor {
    async fn before_request(&self, request: reqwest::RequestBuilder, body: &Vec<u8>, _state: Option<&()>) 
        -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)> {
        let authenticated_request = request.header("Authorization", "Bearer token");
        Ok((authenticated_request, body.clone()))
    }
    
    async fn after_response(&self, response: reqwest::Response, _state: Option<&()>) 
        -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct AuthClient;
```

### 3. Composite Interceptors

```rust
impl AuthClient {
    // Uses both global authentication interceptor and method-level logging interceptor
    #[get(url = "/protected", interceptor = LoggingInterceptor)]
    async fn get_protected_data(&self) -> anyhow::Result<SecretData> {}
}
```

## Error Handling

All generated methods return `anyhow::Result<T>`, providing unified error handling for:

- Network errors
- Serialization/deserialization errors
- HTTP status code errors
- Interceptor errors

## Type Conversion

Swan HTTP supports automatic conversion for various response types:

- `String`: Direct conversion to UTF-8 string
- `Vec<u8>`: Returns raw byte array
- Custom types: JSON deserialization via serde_json

## Logging

The library includes built-in request and response logging using the `log` crate. To enable logging:

```rust
env_logger::init();
```