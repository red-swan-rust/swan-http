# Swan Macro

[![Crates.io](https://img.shields.io/crates/v/swan-macro.svg)](https://crates.io/crates/swan-macro)
[![Documentation](https://docs.rs/swan-macro/badge.svg)](https://docs.rs/swan-macro)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)

🌏 **Languages**: [English](README.md) | [中文](README_CN.md)

Swan Macro is the procedural macro component of the Swan HTTP library, providing declarative HTTP client definition syntax.

## 🌟 Core Features

- **Declarative Client Definition**: Define HTTP clients and methods using macro annotations
- **Automatic Code Generation**: Generate high-performance HTTP client code at compile time
- **Smart Retry Mechanism**: Method-level progressive exponential backoff retry
- **Interceptor Integration**: Seamless integration of global and method-level interceptors
- **Dynamic Parameter Support**: Parameter placeholders in URLs and headers
- **State Injection**: Axum-like application state management

## 📦 Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
swan-macro = "0.2"
swan-common = "0.2"  # Required runtime dependency
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

## 🚀 Quick Start

### Basic Usage

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

// Define HTTP client
#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    // GET request
    #[get(url = "/users/{id}")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}

    // POST request
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    // Request with retry
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self, id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ApiClient::new();
    
    // Use client
    let user = client.get_user(1).await?;
    println!("User: {}", user.name);
    
    Ok(())
}
```

## 🔧 Supported Macros

### `#[http_client]`

Define HTTP client struct:

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = MyInterceptor,  // Optional: global interceptor
    state = AppState              // Optional: application state type
)]
struct ApiClient;
```

### HTTP Method Macros

Supported HTTP methods:

- `#[get(url = "...")]` - GET requests
- `#[post(url = "...", content_type = json)]` - POST requests
- `#[put(url = "...", content_type = json)]` - PUT requests  
- `#[delete(url = "...")]` - DELETE requests

### Method Parameters

```rust
impl ApiClient {
    #[get(
        url = "/users/{id}",                    // Path parameters
        header = "Authorization: Bearer {token}", // Dynamic headers
        retry = "exponential(3, 100ms)",        // Retry strategy
        interceptor = MethodLevelInterceptor    // Method-level interceptor
    )]
    async fn get_user(&self, id: u32, token: String) -> anyhow::Result<User> {}
}
```

## 🔄 Retry Mechanism

Swan HTTP provides intelligent method-level retry mechanisms with exponential backoff and fixed delay strategies.

### Quick Start

```rust
// 📝 Simplest config - exponential retry, 3 attempts, 100ms base delay
#[get(url = "/api", retry = "exponential(3, 100ms)")]

// 📝 Fixed delay - 3 attempts, 1 second each
#[get(url = "/api", retry = "fixed(3, 1s)")]

// 📝 Detailed config - recommended for production
#[get(url = "/api", retry = "exponential(
    max_attempts=5,      // Max 5 attempts (including initial)
    base_delay=200ms,    // Base delay 200ms
    max_delay=30s,       // Max delay 30s
    jitter_ratio=0.1     // 10% random jitter
)")]
```

### Syntax Formats

| Format | Example | Description |
|--------|---------|-------------|
| **Simplified** | `"exponential(3, 100ms)"` | Quick config with positional args |
| **Complete** | `"exponential(max_attempts=3, base_delay=100ms)"` | Named parameters, recommended for production |

### Key Features

- **Auto retry conditions**: 5xx errors, 429 rate limiting, 408 timeout, network errors
- **Idempotency protection**: GET/PUT/DELETE auto retry, POST default no retry
- **Time unit support**: `ms`(milliseconds), `s`(seconds)
- **Compile-time validation**: Configuration errors caught at compile time

> 📖 **Detailed Documentation**: See [Complete Retry Guide](../docs/RETRY_MECHANISM_EN.md) for all parameters, best practices, and troubleshooting

## 🌐 Dynamic Parameters

### URL Parameters

```rust
// Path parameters
#[get(url = "/users/{user_id}/posts/{post_id}")]
async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}

// Query parameters
#[get(url = "/search?q={query}&page={page}")]
async fn search(&self, query: String, page: u32) -> anyhow::Result<Vec<Post>> {}

// Positional parameter reference
#[get(url = "/posts?author={param0}&category={param1}")]
async fn search_by_position(&self, author: String, category: String) -> anyhow::Result<Vec<Post>> {}
```

### Dynamic Headers

```rust
#[get(
    url = "/protected",
    header = "Authorization: Bearer {token}",
    header = "X-User-ID: {user_id}"
)]
async fn get_protected_data(&self, token: String, user_id: u32) -> anyhow::Result<Data> {}
```

## 🔌 Interceptor Integration

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
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, std::borrow::Cow<'a, [u8]>)> {
        let request = request.header("Authorization", "Bearer token");
        Ok((request, std::borrow::Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("Response status: {}", response.status());
        Ok(response)
    }
}

// Use interceptor
#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;
```

## 🏷️ Content Types

Supported content types:

- `content_type = json` - application/json
- `content_type = form_urlencoded` - application/x-www-form-urlencoded
- `content_type = form_multipart` - multipart/form-data

## ⚡ Compile-Time Optimization

Swan Macro generates highly optimized code at compile time:

- **Zero Runtime Overhead**: All configuration determined at compile time
- **Inline Optimization**: Automatically inline small function calls
- **Conditional Compilation**: Remove debug code in release mode
- **Smart Caching**: Interceptor instance reuse

## 🧪 Testing

Run tests:

```bash
cargo test --lib
```

## 📖 Documentation

Detailed API documentation:

```bash
cargo doc --open
```

## 🤝 Use with Swan Common

Swan Macro depends on [Swan Common](https://crates.io/crates/swan-common) for runtime support:

```toml
[dependencies]
swan-macro = "0.2"
swan-common = "0.2"
```

## 📄 License

This project is licensed under the GPL-3.0 License. See the [LICENSE](../LICENSE) file for details.