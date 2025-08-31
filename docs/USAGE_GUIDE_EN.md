# Swan HTTP Usage Guide

🌏 **Languages**: [English](USAGE_GUIDE_EN.md) | [中文](USAGE_GUIDE.md)

## Installation

Add dependencies to your `Cargo.toml`:

```toml
[dependencies]
swan-macro = "0.1.0"
swan-common = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
tokio = { version = "1.0", features = ["macros", "rt-multi-thread"] }
```

## Basic Usage

### 1. Define Data Structures

First define your request and response data structures:

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

### 2. Create HTTP Client

Use the `#[http_client]` macro to define a client:

```rust
use swan_macro::http_client;

#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct ApiClient;
```

### 3. Define HTTP Methods

Define HTTP methods in the client's impl block:

```rust
use swan_macro::{get, post, put, delete};

impl ApiClient {
    // GET request
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    // POST request
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}

    // PUT request
    #[put(url = "/users/1", content_type = json)]
    async fn update_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}

    // DELETE request
    #[delete(url = "/users/1")]
    async fn delete_user(&self) -> anyhow::Result<()> {}
}
```

### 4. Use the Client

```rust
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
    let created = client.create_user(new_user).await?;
    println!("Created successfully: {:?}", created);
    
    Ok(())
}
```

## Advanced Features

### Custom Headers

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

### Different Content Types

```rust
impl ApiClient {
    // JSON (default)
    #[post(url = "/json", content_type = json)]
    async fn post_json(&self, body: MyData) -> anyhow::Result<Response> {}

    // Form URL encoded
    #[post(url = "/form", content_type = form_urlencoded)]
    async fn post_form(&self, body: MyForm) -> anyhow::Result<Response> {}

    // Multipart form
    #[post(url = "/upload", content_type = form_multipart)]
    async fn upload_file(&self, body: FileUpload) -> anyhow::Result<Response> {}
}
```

### Using Interceptors

#### 1. Define Interceptor

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
            println!("Authentication failed, token may need refresh");
        }
        Ok(response)
    }
}
```

#### 2. Global Interceptor

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = AuthInterceptor
)]
struct SecureClient;
```

#### 3. Method-level Interceptor

```rust
impl SecureClient {
    #[get(url = "/data", interceptor = LoggingInterceptor)]
    async fn get_with_logging(&self) -> anyhow::Result<Data> {}
}
```

## Error Handling

Swan HTTP uses `anyhow::Result<T>` for unified error handling:

```rust
impl ApiClient {
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}
}

// Usage
async fn example() {
    let client = ApiClient::new();
    
    match client.get_user().await {
        Ok(user) => println!("Success: {:?}", user),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

Common error types:
- Network connection errors
- HTTP status code errors (4xx, 5xx)
- Serialization/deserialization errors
- Interceptor processing errors

## Response Type Handling

Swan HTTP supports multiple response types:

```rust
impl ApiClient {
    // JSON object
    #[get(url = "/user")]
    async fn get_json(&self) -> anyhow::Result<User> {}

    // Raw string
    #[get(url = "/text")]
    async fn get_text(&self) -> anyhow::Result<String> {}

    // Raw bytes
    #[get(url = "/binary")]
    async fn get_bytes(&self) -> anyhow::Result<Vec<u8>> {}

    // Any JSON value
    #[get(url = "/dynamic")]
    async fn get_value(&self) -> anyhow::Result<serde_json::Value> {}
}
```

## Best Practices

### 1. Structure Organization

```rust
// Recommended: organize clients by functionality
#[http_client(base_url = "https://api.example.com")]
struct UserApiClient;

#[http_client(base_url = "https://api.example.com")]
struct ProductApiClient;
```

### 2. Error Handling

```rust
// Recommended: use ? operator for error propagation
async fn example() -> anyhow::Result<()> {
    let client = ApiClient::new();
    let user = client.get_user().await?;
    let products = client.get_products().await?;
    // ...
    Ok(())
}
```

### 3. Interceptor Design

```rust
// Recommended: keep interceptors simple and focused
#[derive(Default)]
struct RetryInterceptor;

#[async_trait]
impl SwanInterceptor for RetryInterceptor {
    // Only handle retry logic, nothing else
}
```

## Debugging

Enable logging to debug requests and responses:

```rust
// At the beginning of main function
env_logger::init();

// Or use custom log level
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
```

This will display detailed request and response information.