# Swan HTTP - Declarative Rust HTTP Client

[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)

🌏 **Languages**: [English](README.md) | [中文](README_CN.md)

Swan HTTP is a modern, declarative Rust HTTP client library that provides an elegant API design through procedural macros.

## 📦 Crates

Swan HTTP consists of two independent crates:

- **[swan-macro](https://crates.io/crates/swan-macro)** [![Crates.io](https://img.shields.io/crates/v/swan-macro.svg)](https://crates.io/crates/swan-macro) - Procedural macro component
- **[swan-common](https://crates.io/crates/swan-common)** [![Crates.io](https://img.shields.io/crates/v/swan-common.svg)](https://crates.io/crates/swan-common) - Core runtime component

## 🌟 Features

- **Declarative Design**: Define HTTP clients and methods using macro annotations
- **Type Safety**: Full Rust type safety with compile-time error checking
- **Interceptor Support**: Flexible global and method-level interceptor system
- **🆕 State Injection**: Axum-like application state management with dependency injection
- **🆕 Dynamic Parameters**: Parameter placeholders in URLs and headers, supporting `{param_name}` and `{param0}` syntax
- **🔄 Smart Retry**: Method-level progressive exponential backoff retry with idempotency protection and intelligent retry conditions
- **Multiple Content Types**: Support for JSON, form, and multipart form data
- **Async-First**: Tokio-based async design
- **High-Performance Optimization**: Zero-copy, interceptor caching, conditional compilation optimization
- **Modular Architecture**: Clear module separation for easy maintenance and extension

## 🚀 Quick Start

Add the following to your `Cargo.toml`:

```toml
[dependencies]
swan-macro = "0.2"   # Procedural macro component
swan-common = "0.2"  # Core runtime component
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

> **Note**: You need to add both `swan-macro` and `swan-common` dependencies to use Swan HTTP properly.

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
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;

impl ApiClient {
    // GET request
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    // POST request
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    // GET request with retry
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self, id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = ApiClient::new();
    
    // Get user
    let user = client.get_user().await?;
    println!("User: {:?}", user);
    
    // Create user
    let new_user = CreateUserRequest {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };
    let created_user = client.create_user(new_user).await?;
    println!("Created user: {:?}", created_user);
    
    Ok(())
}
```

## 🔧 Advanced Features

### 🔄 Retry Mechanism

Swan HTTP provides powerful method-level retry functionality with intelligent exponential backoff algorithms:

```rust
impl ApiClient {
    // Basic exponential retry: max 3 attempts, base delay 100ms
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    // Detailed configuration: custom all parameters
    #[get(url = "/external/api", retry = "exponential(
        max_attempts=5,
        base_delay=200ms,
        max_delay=30s,
        exponential_base=2.0,
        jitter_ratio=0.1,
        idempotent_only=true
    )")]
    async fn call_external_api(&self) -> anyhow::Result<Data> {}
    
    // Fixed delay retry: for stable services
    #[get(url = "/stable/service", retry = "fixed(max_attempts=4, delay=500ms)")]
    async fn call_stable_service(&self) -> anyhow::Result<Data> {}
}
```

**Retry Features:**
- **Smart Retry Conditions**: Automatically retry 5xx errors, 429 rate limiting, 408 timeouts
- **Idempotency Protection**: Only retry safe GET/PUT/DELETE methods by default
- **Exponential Backoff**: Avoid server overload with customizable growth rate
- **Random Jitter**: Prevent thundering herd effect by spreading retry times
- **Flexible Configuration**: Support both simplified and detailed configuration syntax

For detailed retry mechanism documentation, see: [docs/retry_mechanism.md](docs/retry_mechanism.md)

### Interceptors

Interceptors allow you to perform custom processing before sending requests and after receiving responses:

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
        context: Option<&(dyn Any + Send + Sync)>, // 👈 State context
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let modified_request = request.header("Authorization", "Bearer token");
        // Zero-copy optimization: directly borrow request body to avoid cloning
        Ok((modified_request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>, // 👈 State context
    ) -> anyhow::Result<reqwest::Response> {
        println!("Response status: {}", response.status());
        Ok(response)
    }
}

// Use global interceptor
#[http_client(base_url = "https://api.example.com", interceptor = AuthInterceptor)]
struct SecureApiClient;

impl SecureApiClient {
    // Use method-level interceptor (will stack with global interceptor)
    #[get(url = "/protected", interceptor = LoggingInterceptor)]
    async fn get_protected_data(&self) -> anyhow::Result<serde_json::Value> {}
}
```

### 🆕 State Injection

Swan HTTP supports Axum-like application state management for dependency injection scenarios:

```rust
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// 1. Define application state
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

// 2. Create state-aware interceptor
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
        
        // Get state from context
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

// 3. Declare state type
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState  // 👈 Declare state type
)]
struct StatefulApiClient;

impl StatefulApiClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
}

// 4. Use method chaining to inject state
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = AppState::new();
    
    let client = StatefulApiClient::new()
        .with_state(app_state); // 👈 Inject state
    
    let users = client.get_users().await?;
    Ok(())
}
```

For detailed state injection documentation, see: [docs/STATE_INJECTION.md](docs/STATE_INJECTION.md)

### Supported HTTP Methods

- `#[get]` - GET requests
- `#[post]` - POST requests  
- `#[put]` - PUT requests
- `#[delete]` - DELETE requests

### Content Types

- `json` - application/json
- `form_urlencoded` - application/x-www-form-urlencoded
- `form_multipart` - multipart/form-data

### 🆕 Dynamic Parameters

Support parameter placeholders in URLs and headers with runtime replacement:

```rust
impl ApiClient {
    // URL path parameters
    #[get(url = "/users/{user_id}/posts/{post_id}")]
    async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}
    
    // Query parameters
    #[get(url = "/search?q={query}&page={page}")]
    async fn search(&self, query: String, page: u32) -> anyhow::Result<Vec<Post>> {}
    
    // Dynamic header values
    #[post(
        url = "/users/{user_id}/posts",
        content_type = json,
        header = "Authorization: Bearer {auth_token}",
        header = "X-User-ID: {user_id}"
    )]
    async fn create_post(&self, user_id: u32, auth_token: String, body: CreatePostRequest) -> anyhow::Result<Post> {}
    
    // Positional parameter reference (param0, param1, ...)
    #[get(
        url = "/posts?author={param0}&category={param1}",
        header = "X-Author: {param0}",
        header = "X-Category: {param1}"
    )]
    async fn search_by_position(&self, author: String, category: String) -> anyhow::Result<Vec<Post>> {}
}
```

**Placeholder Syntax:**
- `{param_name}` - Reference by parameter name
- `{param0}`, `{param1}` - Reference by parameter position (starting from 0, excluding self parameter)

### Custom Headers

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

## 📁 Project Architecture

The refactored project adopts a clear modular architecture:

```
swan-http/
├── swan-common/          # Core types and utilities
│   ├── types/           # HTTP methods, content types, etc.
│   ├── parsing/         # Macro parameter parsing logic  
│   └── interceptor/     # Interceptor trait definitions
├── swan-macro/          # Procedural macro implementation
│   ├── generator/       # Code generation logic
│   ├── conversion/      # Type conversion logic
│   ├── request/         # Request handling logic
│   └── error/           # Error handling logic
├── tests/               # Integration tests
└── examples/            # Usage examples
```

This modular design solves the "changing one affects all" problem of the original code, with each module having clear responsibility boundaries.

## 🔍 Refactoring Improvements

Compared to the pre-refactoring version, the new architecture has the following advantages:

1. **Separation of Concerns**: Each module handles specific functionality, reducing coupling
2. **Easy Maintenance**: Modifying one feature doesn't affect other unrelated features
3. **Easy Testing**: Each module can be tested independently
4. **Easy Extension**: New features can be added independently to corresponding modules
5. **Complete Documentation**: Each module and function has comprehensive documentation

## ⚡ Performance Optimization

Swan HTTP library implements multiple performance optimization techniques:

### 1. Interceptor Object Pooling/Caching
- Use `InterceptorCache` to avoid repeatedly creating interceptor instances
- Use `Arc<T>` to share interceptors, reducing memory allocation overhead
- Client-level caching to avoid Box boxing costs

### 2. Zero-Copy Optimization
- Unified `SwanInterceptor` trait uses `Cow<[u8]>` to avoid unnecessary memory copying
- Only clone data when actually needed to modify request body
- Significantly reduce memory allocation and copy overhead

### 3. Lazy Serialization Optimization
- Intelligently detect if request body serialization is needed
- GET/DELETE requests skip serialization process
- Only serialize when interceptors need to access request body

### 4. Conditional Compilation Optimization
- Use `#[cfg(debug_assertions)]` to completely remove debug code in release mode
- Compile-time optimization with zero runtime overhead
- Significantly reduce release version code size

These optimizations ensure Swan HTTP maintains excellent performance while preserving full functionality.

### Performance Optimization Usage Tips

1. **Interceptor Design**: When implementing interceptors, prioritize using `Cow::Borrowed(request_body)` to avoid unnecessary cloning
2. **Dependency Management**: Add `env_logger` or other log implementations in your project to enable debug logging
3. **Release Builds**: Use `cargo build --release` for best performance, debug code will be completely removed
4. **Complex APIs**: Reference `complex_api_example.rs` to understand how to handle complex authentication and headers for enterprise APIs

## 🧪 Running Tests

```bash
# Run all unit tests
cargo test --lib

# Run integration tests
cargo test --test integration_test

# Run examples
cargo run --example basic_usage           # Basic usage example (includes state injection)
cargo run --example interceptor_usage     # Interceptor usage example  
cargo run --example dynamic_params_example # 🆕 Dynamic parameters example (URL and header placeholders)
cargo run --example complex_api_example   # Enterprise API example (performance optimization + state management)
cargo run --example state_injection_example # 🆕 Complete state injection example
cargo run --example simple_retry_test     # 🔄 Simple retry functionality test
cargo run --example retry_integration_test # 🔄 Retry mechanism integration test
```

## 📖 API Documentation

### Online Documentation

- **[swan-macro docs](https://docs.rs/swan-macro)** - Procedural macro API documentation
- **[swan-common docs](https://docs.rs/swan-common)** - Core types and interceptor API documentation

### Local Documentation

Detailed API documentation can be generated and viewed with the following commands:

```bash
# Generate documentation for all components
cargo doc --open

# Or generate documentation for specific components
cargo doc --open -p swan-macro
cargo doc --open -p swan-common
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## 📄 License

This project is licensed under the GPL-3.0 License. See the [LICENSE](LICENSE) file for details.