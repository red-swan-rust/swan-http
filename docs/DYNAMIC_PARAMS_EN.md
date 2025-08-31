# Swan HTTP Dynamic Parameters Guide

🌏 **Languages**: [English](DYNAMIC_PARAMS_EN.md) | [中文](DYNAMIC_PARAMS.md)

## Overview

Swan HTTP supports dynamic parameter placeholders in URLs and headers, allowing runtime value substitution based on method parameters. This feature provides REST-like routing flexibility while maintaining compile-time type safety.

## Core Concepts

### 1. Placeholder Syntax

Swan HTTP supports two placeholder reference methods:

- **By name reference**: `{param_name}` - Use the actual parameter name
- **By position reference**: `{param0}`, `{param1}`, `{param2}` - By parameter position index (skipping `self` parameter)

### 2. Supported Scope

Dynamic parameters can be used in:
- **URL paths**: `/users/{user_id}/posts/{post_id}`
- **Query parameters**: `?q={query}&page={page}&limit={limit}`
- **Header values**: `Authorization: Bearer {token}`

## Basic Usage

### 1. URL Path Parameters

```rust
use swan_macro::{http_client, get};

#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    // Single path parameter
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // Multiple path parameters
    #[get(url = "/users/{user_id}/posts/{post_id}")]
    async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}
    
    // Complex path structure
    #[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
    async fn get_team_member(&self, org_id: String, team_id: u32, member_id: u32) -> anyhow::Result<Member> {}
}
```

### 2. Query Parameters

```rust
impl ApiClient {
    // Basic query parameters
    #[get(url = "/search?q={query}&page={page}")]
    async fn search(&self, query: String, page: u32) -> anyhow::Result<SearchResult> {}
    
    // Complex query parameters
    #[get(url = "/posts?userId={user_id}&_page={page}&_limit={limit}&_sort={sort_field}&_order={order}")]
    async fn get_user_posts(
        &self, 
        user_id: u32, 
        page: u32, 
        limit: u32, 
        sort_field: String, 
        order: String
    ) -> anyhow::Result<Vec<Post>> {}
}
```

### 3. Dynamic Headers

```rust
impl ApiClient {
    // Authentication header
    #[get(
        url = "/protected",
        header = "Authorization: Bearer {auth_token}"
    )]
    async fn get_protected_data(&self, auth_token: String) -> anyhow::Result<Data> {}
    
    // Multiple dynamic headers
    #[post(
        url = "/users/{user_id}/posts",
        content_type = json,
        header = "Authorization: Bearer {token}",
        header = "X-User-ID: {user_id}",
        header = "X-Client-Version: {version}",
        header = "X-Request-ID: {request_id}"
    )]
    async fn create_post(
        &self, 
        user_id: u32, 
        token: String, 
        version: String, 
        request_id: String, 
        body: CreatePostRequest
    ) -> anyhow::Result<Post> {}
}
```

### 4. Position-based Reference

```rust
impl ApiClient {
    // Use position index (starting from 0, skipping self)
    #[get(
        url = "/search?author={param0}&category={param1}&tag={param2}",
        header = "X-Search-Author: {param0}",
        header = "X-Search-Category: {param1}"
    )]
    async fn search_by_position(
        &self,
        author: String,    // {param0}
        category: String,  // {param1}  
        tag: String        // {param2}
    ) -> anyhow::Result<Vec<Post>> {}
}
```

## Advanced Usage

### 1. Mixed Reference Methods

```rust
impl ApiClient {
    // Mix name and position references in the same request
    #[get(
        url = "/users/{user_id}/posts?page={page}&limit={param2}",
        header = "X-User-ID: {user_id}",
        header = "X-Page: {page}",
        header = "X-Request-Info: {param0}-{param1}-{param2}" // Combine multiple parameters
    )]
    async fn get_user_posts_advanced(
        &self,
        user_id: u32,      // {user_id} and {param0}
        page: u32,         // {page} and {param1}
        limit: u32         // {param2}
    ) -> anyhow::Result<Vec<Post>> {}
}
```

### 2. Combined with State Injection

```rust
use swan_common::SwanInterceptor;
use async_trait::async_trait;

#[derive(Clone)]
struct AppState {
    base_auth_token: String,
    tenant_id: String,
}

#[derive(Default)]
struct StateAwareInterceptor;

#[async_trait]
impl SwanInterceptor<AppState> for StateAwareInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        state: Option<&AppState>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let mut request = request;
        
        // Get additional auth info from state
        if let Some(state) = state {
            request = request.header("X-Tenant-ID", &state.tenant_id);
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }
    
    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&AppState>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState
)]
struct AdvancedApiClient;

impl AdvancedApiClient {
    // Dynamic parameters + state injection + interceptor
    #[get(
        url = "/tenants/{tenant_id}/users/{user_id}",
        header = "Authorization: Bearer {auth_token}",
        header = "X-Request-Source: {source}"
    )]
    async fn get_tenant_user(
        &self, 
        tenant_id: String, 
        user_id: u32, 
        auth_token: String, 
        source: String
    ) -> anyhow::Result<User> {}
}
```

## Parameter Processing Rules

### 1. Parameter Recognition

- **Skip self**: Parameter index starts from the first non-`self` parameter
- **Body parameter**: The last parameter of POST/PUT methods is typically treated as request body
- **Dynamic parameters**: All parameters except the body parameter can be used for placeholder substitution

### 2. Type Support

Dynamic parameters support all types that implement `std::fmt::Display`:

```rust
impl ApiClient {
    #[get(url = "/users/{user_id}/score/{score}/active/{is_active}")]
    async fn get_user_status(
        &self,
        user_id: u32,        // Numeric type
        score: f64,          // Float type
        is_active: bool      // Boolean type
    ) -> anyhow::Result<UserStatus> {}
}
```

### 3. Error Handling

If a placeholder references a non-existent parameter, a compile-time error will occur:

```rust
impl ApiClient {
    #[get(url = "/users/{nonexistent_param}")] // ❌ Compile error
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

## Performance Considerations

### 1. Compile-time Processing

- **Zero runtime overhead**: Placeholder substitution is completed at compile time
- **Type safety**: Compile-time verification of parameter existence and type matching
- **No reflection**: No runtime reflection, maintaining high performance

### 2. String Formatting

```rust
// Before compilation (macro input)
#[get(url = "/users/{user_id}/posts/{post_id}")]
async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}

// After compilation (generated code)
pub async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {
    let full_url = format!("{}{}", self.base_url, format!("/users/{}/posts/{}", user_id, post_id));
    // ... rest of request handling code
}
```

## Best Practices

### 1. Naming Conventions

```rust
// ✅ Recommended: Use clear parameter names
#[get(url = "/users/{user_id}/orders/{order_id}")]
async fn get_user_order(&self, user_id: u32, order_id: u32) -> anyhow::Result<Order> {}

// ❌ Avoid: Use ambiguous parameter names
#[get(url = "/users/{id1}/orders/{id2}")]
async fn get_user_order(&self, id1: u32, id2: u32) -> anyhow::Result<Order> {}
```

### 2. Parameter Order

```rust
// ✅ Recommended: Parameter order consistent with URL appearance order
#[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
async fn get_member(&self, org_id: String, team_id: u32, member_id: u32) -> anyhow::Result<Member> {}

// ✅ Also acceptable: Different order, use explicit parameter names
#[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
async fn get_member(&self, member_id: u32, team_id: u32, org_id: String) -> anyhow::Result<Member> {}
```

### 3. Complex Parameter Handling

```rust
impl ApiClient {
    // For complex queries, consider using structs
    #[derive(serde::Serialize)]
    struct SearchParams {
        user_id: u32,
        category: String,
        page: u32,
        limit: u32,
    }
    
    // Then destructure in the method
    async fn search_with_params(&self, params: SearchParams) -> anyhow::Result<SearchResult> {
        self.search_advanced(params.user_id, params.category, params.page, params.limit).await
    }
    
    #[get(url = "/search?userId={user_id}&category={category}&page={page}&limit={limit}")]
    async fn search_advanced(
        &self, 
        user_id: u32, 
        category: String, 
        page: u32, 
        limit: u32
    ) -> anyhow::Result<SearchResult> {}
}
```

## Usage Examples

### Complete REST API Client

```rust
use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post, put, delete};

#[derive(Debug, Deserialize, Serialize)]
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

#[http_client(base_url = "https://api.example.com")]
struct UserApiClient;

impl UserApiClient {
    // Get user list (paginated)
    #[get(url = "/users?page={page}&limit={limit}")]
    async fn list_users(&self, page: u32, limit: u32) -> anyhow::Result<Vec<User>> {}
    
    // Get specific user
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // Create user (with authentication)
    #[post(
        url = "/users",
        content_type = json,
        header = "Authorization: Bearer {auth_token}",
        header = "X-Client-ID: {client_id}"
    )]
    async fn create_user(
        &self, 
        auth_token: String, 
        client_id: String, 
        body: CreateUserRequest
    ) -> anyhow::Result<User> {}
    
    // Update user
    #[put(
        url = "/users/{user_id}",
        content_type = json,
        header = "Authorization: Bearer {auth_token}",
        header = "X-User-ID: {user_id}"  // Reuse parameter
    )]
    async fn update_user(
        &self, 
        user_id: u32, 
        auth_token: String, 
        body: CreateUserRequest
    ) -> anyhow::Result<User> {}
    
    // Delete user
    #[delete(
        url = "/users/{user_id}",
        header = "Authorization: Bearer {auth_token}",
        header = "X-Delete-Reason: {reason}"
    )]
    async fn delete_user(
        &self, 
        user_id: u32, 
        auth_token: String, 
        reason: String
    ) -> anyhow::Result<()> {}
}

// Usage example
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UserApiClient::new();
    
    // All parameters will be automatically substituted into URL and headers
    let user = client.get_user(123).await?;
    let users = client.list_users(1, 10).await?;
    
    let new_user = CreateUserRequest {
        name: "John Doe".to_string(),
        email: "john@example.com".to_string(),
    };
    
    let created_user = client.create_user(
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string(),
        "client-123".to_string(),
        new_user
    ).await?;
    
    Ok(())
}
```

### Position Reference Example

```rust
impl ApiClient {
    // Use position index for more flexible parameter ordering
    #[get(
        url = "/search?q={param0}&category={param1}&page={param2}",
        header = "X-Search: {param0}",
        header = "X-Category: {param1}",
        header = "X-Page: {param2}"
    )]
    async fn search_by_position(
        &self,
        query: String,     // param0
        category: String,  // param1
        page: u32          // param2
    ) -> anyhow::Result<SearchResult> {}
}
```

## Error Handling and Debugging

### 1. Compile-time Errors

```rust
impl ApiClient {
    // ❌ This will cause compile error: parameter 'missing_param' not found
    #[get(url = "/users/{missing_param}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // ❌ Position index out of range will also cause compile error
    #[get(url = "/users/{param5}")]  // Only param0 is available
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

### 2. Debugging Tips

Enable logging to see generated URLs and headers:

```rust
use log::debug;

// Add debug logging in interceptor
#[async_trait]
impl SwanInterceptor for DebugInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        debug!("Request URL and headers will be dynamically replaced");
        Ok((request, Cow::Borrowed(request_body)))
    }
    
    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}
```

### 3. Runtime Verification

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init(); // Enable logging
    
    let client = ApiClient::new();
    
    // Verify URL generation
    println!("Calling get_user(123)...");
    let user = client.get_user(123).await?; // URL: /users/123
    
    println!("Calling search(\"rust\", 1)...");
    let results = client.search("rust".to_string(), 1).await?; // URL: /search?q=rust&page=1
    
    Ok(())
}
```

## Integration with Other Features

### 1. Interceptor Compatibility

Dynamic parameters are fully compatible with interceptors:

```rust
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // Dynamic parameter substitution happens before interceptor call
        // So the request here already contains substituted URL and headers
        println!("Request has completed dynamic parameter substitution");
        Ok((request, Cow::Borrowed(request_body)))
    }
    
    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[http_client(
    base_url = "https://api.example.com",
    interceptor = AuthInterceptor
)]
struct ApiClient;

impl ApiClient {
    #[get(
        url = "/users/{user_id}",
        header = "X-User-ID: {user_id}"
    )]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

### 2. State Injection Compatibility

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState
)]
struct StatefulApiClient;

impl StatefulApiClient {
    // State injection + dynamic parameters work together
    #[get(
        url = "/tenants/{tenant_id}/users/{user_id}",
        header = "X-Tenant-ID: {tenant_id}",
        header = "X-User-ID: {user_id}"
    )]
    async fn get_tenant_user(&self, tenant_id: String, user_id: u32) -> anyhow::Result<User> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = AppState::new();
    let client = StatefulApiClient::new().with_state(app_state);
    
    // Dynamic parameter substitution + state access both work normally
    let user = client.get_tenant_user("tenant-123".to_string(), 456).await?;
    
    Ok(())
}
```

## Limitations and Notes

### 1. Parameter Constraints

- **Type requirement**: Parameters must implement `std::fmt::Display` trait
- **Lifetime**: Parameter values must be valid during request execution
- **Body parameter**: The last parameter of POST/PUT methods is recognized as request body, cannot be used for placeholders

### 2. Placeholder Format

- **Fixed format**: Must use `{param_name}` or `{paramN}` format
- **Case sensitive**: Parameter names must match exactly
- **No nesting**: Nested placeholders like `{{param}}` are not supported

### 3. Performance Impact

- **String allocation**: String formatting is performed on each request
- **Compile time**: Complex parameter substitution may slightly increase compile time
- **Runtime efficiency**: Formatting operation performance impact is usually negligible

## Migration Guide

### From Static URLs

```rust
// Before: static URL
#[get(url = "/users/1")]
async fn get_user(&self) -> anyhow::Result<User> {}

// After: dynamic parameters
#[get(url = "/users/{user_id}")]
async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
```

### From String Concatenation

```rust
// Before: manual concatenation
impl ApiClient {
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {
        let url = format!("{}/users/{}", self.base_url, user_id);
        // Manual request construction...
    }
}

// After: declarative dynamic parameters
impl ApiClient {
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

## Troubleshooting

### Common Issues

1. **Compile error**: "Parameter 'xxx' not found"
   - Check parameter name spelling
   - Confirm parameter exists in method signature

2. **Type error**: "Display trait not implemented"
   - Ensure parameter type implements `Display` trait
   - Consider using `.to_string()` conversion

3. **Body parameter conflict**:
   - Last parameter of POST/PUT methods is recognized as body
   - If needed in URL, adjust parameter order

### Debugging Steps

1. **Verify parameter mapping**:
   ```rust
   // Add debug output
   println!("user_id: {}, post_id: {}", user_id, post_id);
   ```

2. **Check generated code**:
   ```bash
   # View expanded macro code
   cargo expand --example your_example
   ```

3. **Enable verbose logging**:
   ```rust
   env_logger::init();
   // Or use other logging libraries
   ```

## Complete Examples

For detailed usage examples, see:
- `examples/dynamic_params_example.rs` - Complete dynamic parameters demo
- `examples/basic_usage.rs` - Simple dynamic parameters in basic usage
- `examples/complex_api_example.rs` - Parameter usage in enterprise scenarios

These examples demonstrate the application of dynamic parameters in real API calls, including REST-style path parameters, complex query parameters, authentication headers, and other scenarios.