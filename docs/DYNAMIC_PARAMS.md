# Swan HTTP 动态参数指南

## 概述

Swan HTTP 支持在 URL 和 header 中使用动态参数占位符，允许运行时根据方法参数动态替换值。这个功能提供了类似 REST 路由的灵活性，同时保持编译时类型安全。

## 核心概念

### 1. 占位符语法

Swan HTTP 支持两种占位符引用方式：

- **按名称引用**: `{param_name}` - 使用参数的实际名称
- **按位置引用**: `{param0}`, `{param1}`, `{param2}` - 按参数位置索引（跳过 `self` 参数）

### 2. 支持范围

动态参数可以用在：
- **URL 路径**: `/users/{user_id}/posts/{post_id}`
- **查询参数**: `?q={query}&page={page}&limit={limit}`
- **Header 值**: `Authorization: Bearer {token}`

## 基础用法

### 1. URL 路径参数

```rust
use swan_macro::{http_client, get};

#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    // 单个路径参数
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // 多个路径参数
    #[get(url = "/users/{user_id}/posts/{post_id}")]
    async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}
    
    // 复杂路径结构
    #[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
    async fn get_team_member(&self, org_id: String, team_id: u32, member_id: u32) -> anyhow::Result<Member> {}
}
```

### 2. 查询参数

```rust
impl ApiClient {
    // 基础查询参数
    #[get(url = "/search?q={query}&page={page}")]
    async fn search(&self, query: String, page: u32) -> anyhow::Result<SearchResult> {}
    
    // 复杂查询参数
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

### 3. 动态 Header

```rust
impl ApiClient {
    // 认证 header
    #[get(
        url = "/protected",
        header = "Authorization: Bearer {auth_token}"
    )]
    async fn get_protected_data(&self, auth_token: String) -> anyhow::Result<Data> {}
    
    // 多个动态 header
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

### 4. 按位置引用

```rust
impl ApiClient {
    // 使用位置索引（从0开始，跳过self）
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

## 高级用法

### 1. 混合引用方式

```rust
impl ApiClient {
    // 在同一个请求中混合使用名称和位置引用
    #[get(
        url = "/users/{user_id}/posts?page={page}&limit={param2}",
        header = "X-User-ID: {user_id}",
        header = "X-Page: {page}",
        header = "X-Request-Info: {param0}-{param1}-{param2}" // 组合多个参数
    )]
    async fn get_user_posts_advanced(
        &self,
        user_id: u32,      // {user_id} 和 {param0}
        page: u32,         // {page} 和 {param1}
        limit: u32         // {param2}
    ) -> anyhow::Result<Vec<Post>> {}
}
```

### 2. 与状态注入结合

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
        
        // 从状态获取额外的认证信息
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
    // 动态参数 + 状态注入 + 拦截器
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

## 参数处理规则

### 1. 参数识别

- **跳过 self**: 参数索引从第一个非 `self` 参数开始计算
- **Body 参数**: POST/PUT 方法的最后一个参数通常被视为请求体
- **动态参数**: 除 body 参数外的所有参数都可用于占位符替换

### 2. 类型支持

动态参数支持所有实现了 `std::fmt::Display` 的类型：

```rust
impl ApiClient {
    #[get(url = "/users/{user_id}/score/{score}/active/{is_active}")]
    async fn get_user_status(
        &self,
        user_id: u32,        // 数字类型
        score: f64,          // 浮点数
        is_active: bool      // 布尔类型
    ) -> anyhow::Result<UserStatus> {}
}
```

### 3. 错误处理

如果占位符引用的参数不存在，会产生编译时错误：

```rust
impl ApiClient {
    #[get(url = "/users/{nonexistent_param}")] // ❌ 编译错误
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

## 性能考虑

### 1. 编译时处理

- **零运行时开销**: 占位符替换在编译时完成
- **类型安全**: 编译时验证参数存在性和类型匹配
- **无反射**: 不使用运行时反射，保持高性能

### 2. 字符串格式化

```rust
// 编译前（宏输入）
#[get(url = "/users/{user_id}/posts/{post_id}")]
async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {}

// 编译后（生成的代码）
pub async fn get_user_post(&self, user_id: u32, post_id: u32) -> anyhow::Result<Post> {
    let full_url = format!("{}{}", self.base_url, format!("/users/{}/posts/{}", user_id, post_id));
    // ... 其余请求处理代码
}
```

## 最佳实践

### 1. 命名约定

```rust
// ✅ 推荐：使用清晰的参数名
#[get(url = "/users/{user_id}/orders/{order_id}")]
async fn get_user_order(&self, user_id: u32, order_id: u32) -> anyhow::Result<Order> {}

// ❌ 避免：使用模糊的参数名
#[get(url = "/users/{id1}/orders/{id2}")]
async fn get_user_order(&self, id1: u32, id2: u32) -> anyhow::Result<Order> {}
```

### 2. 参数顺序

```rust
// ✅ 推荐：参数顺序与URL中出现顺序一致
#[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
async fn get_member(&self, org_id: String, team_id: u32, member_id: u32) -> anyhow::Result<Member> {}

// ✅ 也可以：不同顺序，使用明确的参数名
#[get(url = "/orgs/{org_id}/teams/{team_id}/members/{member_id}")]
async fn get_member(&self, member_id: u32, team_id: u32, org_id: String) -> anyhow::Result<Member> {}
```

### 3. 复杂参数处理

```rust
impl ApiClient {
    // 对于复杂的查询，考虑使用结构体
    #[derive(serde::Serialize)]
    struct SearchParams {
        user_id: u32,
        category: String,
        page: u32,
        limit: u32,
    }
    
    // 然后在方法中解构
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

## 使用示例

### 完整的 REST API 客户端

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
    // 获取用户列表（分页）
    #[get(url = "/users?page={page}&limit={limit}")]
    async fn list_users(&self, page: u32, limit: u32) -> anyhow::Result<Vec<User>> {}
    
    // 获取特定用户
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // 创建用户（带认证）
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
    
    // 更新用户
    #[put(
        url = "/users/{user_id}",
        content_type = json,
        header = "Authorization: Bearer {auth_token}",
        header = "X-User-ID: {user_id}"  // 复用参数
    )]
    async fn update_user(
        &self, 
        user_id: u32, 
        auth_token: String, 
        body: CreateUserRequest
    ) -> anyhow::Result<User> {}
    
    // 删除用户
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

// 使用示例
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UserApiClient::new();
    
    // 所有参数会自动替换到URL和header中
    let user = client.get_user(123).await?;
    let users = client.list_users(1, 10).await?;
    
    let new_user = CreateUserRequest {
        name: "张三".to_string(),
        email: "zhangsan@example.com".to_string(),
    };
    
    let created_user = client.create_user(
        "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9".to_string(),
        "client-123".to_string(),
        new_user
    ).await?;
    
    Ok(())
}
```

### 位置引用示例

```rust
impl ApiClient {
    // 使用位置索引，参数顺序可以更灵活
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

## 错误处理和调试

### 1. 编译时错误

```rust
impl ApiClient {
    // ❌ 这会产生编译错误：参数 'missing_param' 未找到
    #[get(url = "/users/{missing_param}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
    
    // ❌ 位置索引超出范围也会产生编译错误
    #[get(url = "/users/{param5}")]  // 只有 param0 可用
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

### 2. 调试技巧

启用日志来查看生成的URL和header：

```rust
use log::debug;

// 在拦截器中添加调试日志
#[async_trait]
impl SwanInterceptor<()> for DebugInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        debug!("请求URL和header将被动态替换");
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

### 3. 运行时验证

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init(); // 启用日志
    
    let client = ApiClient::new();
    
    // 验证URL生成
    println!("调用 get_user(123)...");
    let user = client.get_user(123).await?; // URL: /users/123
    
    println!("调用 search(\"rust\", 1)...");
    let results = client.search("rust".to_string(), 1).await?; // URL: /search?q=rust&page=1
    
    Ok(())
}
```

## 与其他功能集成

### 1. 拦截器兼容性

动态参数与拦截器完全兼容：

```rust
#[derive(Default)]
struct AuthInterceptor;

#[async_trait]
impl SwanInterceptor<()> for AuthInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // 动态参数替换发生在拦截器调用之前
        // 所以这里的request已经包含了替换后的URL和header
        println!("请求已经完成动态参数替换");
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

### 2. 状态注入兼容性

```rust
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState
)]
struct StatefulApiClient;

impl StatefulApiClient {
    // 状态注入 + 动态参数同时工作
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
    
    // 动态参数替换 + 状态访问都正常工作
    let user = client.get_tenant_user("tenant-123".to_string(), 456).await?;
    
    Ok(())
}
```

## 限制和注意事项

### 1. 参数约束

- **类型要求**: 参数必须实现 `std::fmt::Display` trait
- **生命周期**: 参数值在请求执行期间必须有效
- **Body 参数**: POST/PUT 方法的最后一个参数会被识别为请求体，不能用于占位符

### 2. 占位符格式

- **格式固定**: 必须使用 `{param_name}` 或 `{paramN}` 格式
- **大小写敏感**: 参数名大小写必须完全匹配
- **无嵌套**: 不支持嵌套占位符如 `{{param}}`

### 3. 性能影响

- **字符串分配**: 每次请求都会进行字符串格式化
- **编译时间**: 复杂的参数替换可能略微增加编译时间
- **运行时效率**: 格式化操作的性能影响通常可以忽略不计

## 迁移指南

### 从静态URL迁移

```rust
// 之前：静态URL
#[get(url = "/users/1")]
async fn get_user(&self) -> anyhow::Result<User> {}

// 之后：动态参数
#[get(url = "/users/{user_id}")]
async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
```

### 从字符串拼接迁移

```rust
// 之前：手动拼接
impl ApiClient {
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {
        let url = format!("{}/users/{}", self.base_url, user_id);
        // 手动构建请求...
    }
}

// 之后：声明式动态参数
impl ApiClient {
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}
}
```

## 故障排除

### 常见问题

1. **编译错误**: "Parameter 'xxx' not found"
   - 检查参数名是否正确拼写
   - 确认参数存在于方法签名中

2. **类型错误**: "Display trait not implemented"
   - 确保参数类型实现了 `Display` trait
   - 考虑使用 `.to_string()` 转换

3. **Body 参数冲突**:
   - POST/PUT 方法的最后一个参数会被识别为body
   - 如需在URL中使用该参数，请调整参数顺序

### 调试步骤

1. **验证参数映射**:
   ```rust
   // 添加调试输出
   println!("user_id: {}, post_id: {}", user_id, post_id);
   ```

2. **检查生成的代码**:
   ```bash
   # 查看宏展开后的代码
   cargo expand --example your_example
   ```

3. **启用详细日志**:
   ```rust
   env_logger::init();
   // 或使用其他日志库
   ```

## 完整示例

详细的使用示例请参考：
- `examples/dynamic_params_example.rs` - 完整的动态参数演示
- `examples/basic_usage.rs` - 基础用法中的简单动态参数
- `examples/complex_api_example.rs` - 企业级场景中的参数使用

这些示例展示了动态参数在真实API调用中的应用，包括REST风格的路径参数、复杂查询参数、认证header等场景。