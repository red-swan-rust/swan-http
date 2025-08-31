# Swan HTTP State Injection Guide

🌏 **Languages**: [English](STATE_INJECTION_EN.md) | [中文](STATE_INJECTION.md)

## Overview

Swan HTTP supports application state injection, allowing interceptors to access shared state (such as database connection pools, caches, configuration, etc.). This feature is similar to Axum's app state but specifically designed for HTTP clients.

## Core Concepts

### 1. State Injection Mechanism

- **Declarative configuration**: Declare `state = YourStateType` in the `#[http_client]` macro
- **Chained initialization**: Use `.with_state(state)` method to inject state instance
- **Automatic passing**: Framework automatically passes state as context to interceptors
- **Type safety**: Safe state access through `downcast_ref::<YourStateType>()`

### 2. Interceptor Context Parameter

All interceptor methods include a `context` parameter:

```rust
async fn before_request<'a>(
    &self,
    request: reqwest::RequestBuilder,
    request_body: &'a [u8],
    context: Option<&(dyn Any + Send + Sync)>, // 👈 State is passed here
) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
```

## Basic Usage

### 1. Define Application State

```rust
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

#[derive(Clone)]
struct AppState {
    // Redis cache simulation
    cache: Arc<RwLock<HashMap<String, String>>>,
    // Database connection pool simulation
    db_pool: Arc<RwLock<u32>>,
    // Request counter
    request_counter: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut cache = HashMap::new();
        cache.insert("auth_token".to_string(), "cached-jwt-token-12345".to_string());
        
        Self {
            cache: Arc::new(RwLock::new(cache)),
            db_pool: Arc::new(RwLock::new(10)), // 10 connections
            request_counter: Arc::new(RwLock::new(0)),
        }
    }
    
    pub async fn get_cached_token(&self) -> Option<String> {
        self.cache.read().unwrap().get("auth_token").cloned()
    }
    
    pub async fn increment_counter(&self) -> u64 {
        let mut counter = self.request_counter.write().unwrap();
        *counter += 1;
        *counter
    }
}
```

### 2. Create State-aware Interceptor

```rust
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;

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
        
        // Get application state from context
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                // Use cached token from state
                if let Some(token) = app_state.get_cached_token().await {
                    println!("🔐 Using cached token: {}...", &token[..20]);
                    request = request.header("Authorization", format!("Bearer {}", token));
                    
                    // Update request counter
                    let count = app_state.increment_counter().await;
                    request = request.header("X-Request-Count", count.to_string());
                } else {
                    // fallback to default token
                    request = request.header("Authorization", "Bearer default-token");
                }
            }
        } else {
            // No state fallback
            request = request.header("Authorization", "Bearer no-state-token");
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                let current_count = *app_state.request_counter.read().unwrap();
                println!("📈 State statistics: Currently processed {} requests", current_count);
            }
        }
        
        Ok(response)
    }
}
```

### 3. Configure HTTP Client with State

```rust
use swan_macro::{http_client, get, post};

// Declare state type
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState  // 👈 Declare state type
)]
struct ApiClient;

impl ApiClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
    
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
}
```

### 4. Using Stateful Client

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Create application state
    let app_state = AppState::new();
    
    // 2. Create client and inject state
    let client = ApiClient::new()
        .with_state(app_state); // 👈 Chain call to inject state
    
    // 3. Call API (interceptor will automatically get state)
    let users = client.get_users().await?;
    println!("Retrieved {} users", users.len());
    
    Ok(())
}
```

## Advanced Usage

### 1. Multiple State Types

```rust
// Database state
#[derive(Clone)]
struct DatabaseState {
    pool: Arc<sqlx::Pool<sqlx::Postgres>>,
}

// Cache state
#[derive(Clone)]
struct CacheState {
    redis: Arc<redis::Client>,
}

// Combined state
#[derive(Clone)]
struct AppState {
    db: DatabaseState,
    cache: CacheState,
    metrics: Arc<RwLock<Metrics>>,
}
```

### 2. Conditional State Access

```rust
#[async_trait]
impl SwanInterceptor for MyInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let mut request = request;
        
        if let Some(ctx) = context {
            // Try multiple state types
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                // Handle full application state
                request = self.handle_full_state(request, app_state).await?;
            } else if let Some(db_state) = ctx.downcast_ref::<DatabaseState>() {
                // Only database state
                request = self.handle_db_only(request, db_state).await?;
            } else {
                // Unknown state type
                println!("⚠️ Unknown state type");
            }
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }
    
    // ... other methods
}
```

### 3. State Lifecycle Management

```rust
// Create state when application starts
let app_state = AppState::new().await?;

// Create multiple clients sharing state
let user_client = UserApiClient::new().with_state(app_state.clone());
let order_client = OrderApiClient::new().with_state(app_state.clone());
let product_client = ProductApiClient::new().with_state(app_state.clone());

// State is shared across all clients
tokio::try_join!(
    user_client.get_profile(),
    order_client.get_orders(),
    product_client.get_catalog(),
)?;
```

## Best Practices

### 1. State Design Principles

- **Immutability**: Use `Arc<RwLock<T>>` or `Arc<Mutex<T>>` to ensure thread safety
- **Clone friendly**: State struct should implement `Clone` to support sharing across multiple clients
- **Type clarity**: Create clear state types for different purposes, avoid using generic Any
- **Resource management**: Manage expensive resources (database connections, Redis clients, etc.) in state

### 2. Interceptor State Access

```rust
// ✅ Recommended: Clear type checking
if let Some(ctx) = context {
    if let Some(app_state) = ctx.downcast_ref::<AppState>() {
        // Safe state access
    }
}

// ❌ Avoid: Assuming state always exists
let app_state = context.unwrap().downcast_ref::<AppState>().unwrap();
```

### 3. Error Handling

```rust
async fn before_request<'a>(
    &self,
    request: reqwest::RequestBuilder,
    request_body: &'a [u8],
    context: Option<&(dyn Any + Send + Sync)>,
) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
    let mut request = request;
    
    match context.and_then(|ctx| ctx.downcast_ref::<AppState>()) {
        Some(state) => {
            // Processing logic when state exists
            match state.get_auth_token().await {
                Some(token) => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                None => {
                    // Token fetch failed, use fallback
                    request = request.header("Authorization", "Bearer fallback-token");
                }
            }
        }
        None => {
            // Fallback processing when no state
            request = request.header("Authorization", "Bearer default-token");
        }
    }
    
    Ok((request, Cow::Borrowed(request_body)))
}
```

### 4. Performance Optimization

- **Avoid frequent locking**: Try to get required data at once, avoid multiple locks
- **Use Cow optimization**: Maintain zero-copy characteristics, only clone when necessary
- **State prewarming**: Preload commonly used data into state during application startup

```rust
// ✅ Efficient: Get multiple values at once
let (token, user_id, config) = {
    let state_guard = app_state.read().unwrap();
    (
        state_guard.auth_token.clone(),
        state_guard.current_user_id,
        state_guard.api_config.clone(),
    )
};

// ❌ Inefficient: Multiple locks
let token = app_state.read().unwrap().auth_token.clone();
let user_id = app_state.read().unwrap().current_user_id;
let config = app_state.read().unwrap().api_config.clone();
```

## Common Use Cases

### 1. Authentication Token Management

```rust
#[derive(Clone)]
struct AuthState {
    tokens: Arc<RwLock<TokenPool>>,
    refresh_strategy: RefreshStrategy,
}

impl AuthState {
    pub async fn get_valid_token(&self) -> anyhow::Result<String> {
        // Automatically refresh expired tokens
        // Get available token from token pool
        // Handle token rotation logic
    }
}
```

### 2. Database Connection Pool

```rust
#[derive(Clone)]
struct DatabaseState {
    pool: Arc<sqlx::PgPool>,
}

impl DatabaseState {
    pub async fn get_user_permissions(&self, user_id: u64) -> anyhow::Result<Vec<Permission>> {
        // Query user permissions from database
        // Perform permission validation in interceptor
    }
}
```

### 3. Cache System Integration

```rust
#[derive(Clone)]
struct CacheState {
    redis: Arc<redis::aio::ConnectionManager>,
}

impl CacheState {
    pub async fn get_cached_response(&self, key: &str) -> Option<String> {
        // Check if cache has pre-stored response
        // Implement response caching in interceptor
    }
}
```

### 4. Metrics and Monitoring

```rust
#[derive(Clone)]
struct MetricsState {
    metrics: Arc<RwLock<AppMetrics>>,
    prometheus: Arc<prometheus::Registry>,
}

impl MetricsState {
    pub fn record_request(&self, endpoint: &str, method: &str) {
        // Record request metrics
        // Update Prometheus counters
    }
}
```

## Complete Examples

Please refer to the following example files:

- `examples/state_injection_example.rs` - Basic state injection example
- `examples/basic_usage.rs` - Simple state management
- `examples/complex_api_example.rs` - Enterprise-level state management

## Migration Guide

### From Stateless to Stateful

1. **Add state declaration**:
   ```rust
   // Before
   #[http_client(base_url = "...", interceptor = MyInterceptor)]
   struct Client;
   
   // After
   #[http_client(base_url = "...", interceptor = MyInterceptor, state = AppState)]
   struct Client;
   ```

2. **Update interceptor signature**:
   ```rust
   // Before
   async fn before_request<'a>(
       &self,
       request: reqwest::RequestBuilder,
       request_body: &'a [u8],
   ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
   
   // After
   async fn before_request<'a>(
       &self,
       request: reqwest::RequestBuilder,
       request_body: &'a [u8],
       context: Option<&(dyn Any + Send + Sync)>, // 👈 New parameter
   ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
   ```

3. **Update client initialization**:
   ```rust
   // Before
   let client = ApiClient::new();
   
   // After
   let app_state = AppState::new();
   let client = ApiClient::new().with_state(app_state);
   ```

## Important Notes

1. **Thread safety**: State must implement `Send + Sync`
2. **Clone cost**: State should use `Arc` to wrap expensive resources
3. **Type checking**: Use `downcast_ref` for safe type conversion
4. **Fallback mechanism**: Always provide fallback handling for stateless situations
5. **Backward compatibility**: Existing stateless interceptors can continue working by ignoring the context parameter

## Performance Considerations

- **State access overhead**: `downcast_ref` has slight runtime overhead, but is faster than dynamic dispatch
- **Memory usage**: State is shared across all client instances, saving memory
- **Lock contention**: Design state structure reasonably to avoid lock contention
- **Prewarming strategy**: Preload commonly used data during application startup

## Troubleshooting

### Common Errors

1. **downcast failure**: Check if state type matches correctly
2. **Send + Sync error**: Ensure all fields in state are thread-safe
3. **Clone error**: State type must implement `Clone`
4. **Lifetime issues**: Ensure state lifetime is longer than client

### Debugging Tips

```rust
// Debug state passing
if let Some(ctx) = context {
    println!("Received context, type: {:?}", ctx.type_id());
    if let Some(state) = ctx.downcast_ref::<AppState>() {
        println!("Successfully got AppState");
    } else {
        println!("downcast failed");
    }
} else {
    println!("No context received");
}
```