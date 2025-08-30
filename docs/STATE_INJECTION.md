# Swan HTTP State 注入指南

## 概述

Swan HTTP 支持应用状态注入，允许拦截器访问共享状态（如数据库连接池、缓存、配置等）。这个功能类似于 Axum 的 app state，但专门为 HTTP 客户端设计。

## 核心概念

### 1. 状态注入机制

- **声明式配置**: 在 `#[http_client]` 宏中声明 `state = YourStateType`
- **链式初始化**: 使用 `.with_state(state)` 方法注入状态实例
- **自动传递**: 框架自动将状态作为 context 传递给拦截器
- **类型安全**: 通过 `downcast_ref::<YourStateType>()` 安全访问状态

### 2. 拦截器 Context 参数

所有拦截器方法都包含一个 `context` 参数：

```rust
async fn before_request<'a>(
    &self,
    request: reqwest::RequestBuilder,
    request_body: &'a [u8],
    context: Option<&(dyn Any + Send + Sync)>, // 👈 状态通过这里传递
) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
```

## 基础用法

### 1. 定义应用状态

```rust
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

#[derive(Clone)]
struct AppState {
    // Redis 缓存模拟
    cache: Arc<RwLock<HashMap<String, String>>>,
    // 数据库连接池模拟
    db_pool: Arc<RwLock<u32>>,
    // 请求计数器
    request_counter: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut cache = HashMap::new();
        cache.insert("auth_token".to_string(), "cached-jwt-token-12345".to_string());
        
        Self {
            cache: Arc::new(RwLock::new(cache)),
            db_pool: Arc::new(RwLock::new(10)), // 10个连接
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

### 2. 创建状态感知的拦截器

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
        
        // 从 context 获取应用状态
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                // 使用状态中的缓存token
                if let Some(token) = app_state.get_cached_token().await {
                    println!("🔐 使用缓存token: {}...", &token[..20]);
                    request = request.header("Authorization", format!("Bearer {}", token));
                    
                    // 更新请求计数器
                    let count = app_state.increment_counter().await;
                    request = request.header("X-Request-Count", count.to_string());
                } else {
                    // fallback 到默认token
                    request = request.header("Authorization", "Bearer default-token");
                }
            }
        } else {
            // 无状态fallback
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
                println!("📈 State统计: 当前已处理 {} 个请求", current_count);
            }
        }
        
        Ok(response)
    }
}
```

### 3. 配置带状态的HTTP客户端

```rust
use swan_macro::{http_client, get, post};

// 声明状态类型
#[http_client(
    base_url = "https://api.example.com",
    interceptor = StateAwareInterceptor,
    state = AppState  // 👈 声明状态类型
)]
struct ApiClient;

impl ApiClient {
    #[get(url = "/users")]
    async fn get_users(&self) -> anyhow::Result<Vec<User>> {}
    
    #[post(url = "/users", content_type = json)]
    async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
}
```

### 4. 使用带状态的客户端

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. 创建应用状态
    let app_state = AppState::new();
    
    // 2. 创建客户端并注入状态
    let client = ApiClient::new()
        .with_state(app_state); // 👈 链式调用注入状态
    
    // 3. 调用API（拦截器会自动获取状态）
    let users = client.get_users().await?;
    println!("获取到 {} 个用户", users.len());
    
    Ok(())
}
```

## 高级用法

### 1. 多种状态类型

```rust
// 数据库状态
#[derive(Clone)]
struct DatabaseState {
    pool: Arc<sqlx::Pool<sqlx::Postgres>>,
}

// 缓存状态
#[derive(Clone)]
struct CacheState {
    redis: Arc<redis::Client>,
}

// 组合状态
#[derive(Clone)]
struct AppState {
    db: DatabaseState,
    cache: CacheState,
    metrics: Arc<RwLock<Metrics>>,
}
```

### 2. 条件状态访问

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
            // 尝试多种状态类型
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                // 处理完整应用状态
                request = self.handle_full_state(request, app_state).await?;
            } else if let Some(db_state) = ctx.downcast_ref::<DatabaseState>() {
                // 只有数据库状态
                request = self.handle_db_only(request, db_state).await?;
            } else {
                // 未知状态类型
                println!("⚠️ 未知的状态类型");
            }
        }
        
        Ok((request, Cow::Borrowed(request_body)))
    }
    
    // ... 其他方法
}
```

### 3. 状态生命周期管理

```rust
// 应用启动时创建状态
let app_state = AppState::new().await?;

// 创建多个客户端共享状态
let user_client = UserApiClient::new().with_state(app_state.clone());
let order_client = OrderApiClient::new().with_state(app_state.clone());
let product_client = ProductApiClient::new().with_state(app_state.clone());

// 状态在所有客户端间共享
tokio::try_join!(
    user_client.get_profile(),
    order_client.get_orders(),
    product_client.get_catalog(),
)?;
```

## 最佳实践

### 1. 状态设计原则

- **不可变性**: 尽量使用 `Arc<RwLock<T>>` 或 `Arc<Mutex<T>>` 确保线程安全
- **Clone友好**: 状态结构体应该实现 `Clone`，以支持在多个客户端间共享
- **类型明确**: 为不同用途创建明确的状态类型，避免使用泛型Any
- **资源管理**: 在状态中管理昂贵资源（数据库连接、Redis客户端等）

### 2. 拦截器状态访问

```rust
// ✅ 推荐：明确的类型检查
if let Some(ctx) = context {
    if let Some(app_state) = ctx.downcast_ref::<AppState>() {
        // 安全访问状态
    }
}

// ❌ 避免：假设状态一定存在
let app_state = context.unwrap().downcast_ref::<AppState>().unwrap();
```

### 3. 错误处理

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
            // 有状态时的处理逻辑
            match state.get_auth_token().await {
                Some(token) => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                None => {
                    // token获取失败，使用fallback
                    request = request.header("Authorization", "Bearer fallback-token");
                }
            }
        }
        None => {
            // 无状态时的fallback处理
            request = request.header("Authorization", "Bearer default-token");
        }
    }
    
    Ok((request, Cow::Borrowed(request_body)))
}
```

### 4. 性能优化

- **避免频繁锁**: 尽量一次获取所需数据，避免多次加锁
- **使用Cow优化**: 保持零拷贝特性，只在必要时克隆
- **状态预热**: 在应用启动时预先加载常用数据到状态中

```rust
// ✅ 高效：一次获取多个值
let (token, user_id, config) = {
    let state_guard = app_state.read().unwrap();
    (
        state_guard.auth_token.clone(),
        state_guard.current_user_id,
        state_guard.api_config.clone(),
    )
};

// ❌ 低效：多次加锁
let token = app_state.read().unwrap().auth_token.clone();
let user_id = app_state.read().unwrap().current_user_id;
let config = app_state.read().unwrap().api_config.clone();
```

## 常见使用场景

### 1. 认证Token管理

```rust
#[derive(Clone)]
struct AuthState {
    tokens: Arc<RwLock<TokenPool>>,
    refresh_strategy: RefreshStrategy,
}

impl AuthState {
    pub async fn get_valid_token(&self) -> anyhow::Result<String> {
        // 自动刷新过期token
        // 从token池获取可用token
        // 处理token轮换逻辑
    }
}
```

### 2. 数据库连接池

```rust
#[derive(Clone)]
struct DatabaseState {
    pool: Arc<sqlx::PgPool>,
}

impl DatabaseState {
    pub async fn get_user_permissions(&self, user_id: u64) -> anyhow::Result<Vec<Permission>> {
        // 从数据库查询用户权限
        // 在拦截器中进行权限验证
    }
}
```

### 3. 缓存系统集成

```rust
#[derive(Clone)]
struct CacheState {
    redis: Arc<redis::aio::ConnectionManager>,
}

impl CacheState {
    pub async fn get_cached_response(&self, key: &str) -> Option<String> {
        // 检查缓存是否有预存响应
        // 在拦截器中实现响应缓存
    }
}
```

### 4. 指标和监控

```rust
#[derive(Clone)]
struct MetricsState {
    metrics: Arc<RwLock<AppMetrics>>,
    prometheus: Arc<prometheus::Registry>,
}

impl MetricsState {
    pub fn record_request(&self, endpoint: &str, method: &str) {
        // 记录请求指标
        // 更新Prometheus计数器
    }
}
```

## 完整示例

请参考以下示例文件：

- `examples/state_injection_example.rs` - 基础状态注入示例
- `examples/basic_usage.rs` - 简单状态管理
- `examples/complex_api_example.rs` - 企业级状态管理

## 迁移指南

### 从无状态到有状态

1. **添加状态声明**:
   ```rust
   // 之前
   #[http_client(base_url = "...", interceptor = MyInterceptor)]
   struct Client;
   
   // 之后
   #[http_client(base_url = "...", interceptor = MyInterceptor, state = AppState)]
   struct Client;
   ```

2. **更新拦截器签名**:
   ```rust
   // 之前
   async fn before_request<'a>(
       &self,
       request: reqwest::RequestBuilder,
       request_body: &'a [u8],
   ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
   
   // 之后
   async fn before_request<'a>(
       &self,
       request: reqwest::RequestBuilder,
       request_body: &'a [u8],
       context: Option<&(dyn Any + Send + Sync)>, // 👈 新增参数
   ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>
   ```

3. **更新客户端初始化**:
   ```rust
   // 之前
   let client = ApiClient::new();
   
   // 之后
   let app_state = AppState::new();
   let client = ApiClient::new().with_state(app_state);
   ```

## 注意事项

1. **线程安全**: 状态必须实现 `Send + Sync`
2. **克隆成本**: 状态应该使用 `Arc` 包装昂贵资源
3. **类型检查**: 使用 `downcast_ref` 进行安全的类型转换
4. **fallback机制**: 始终为无状态情况提供fallback处理
5. **向后兼容**: 现有的无状态拦截器可以通过忽略 context 参数继续工作

## 性能考虑

- **状态访问开销**: `downcast_ref` 有轻微运行时开销，但比动态分发快
- **内存使用**: 状态在所有客户端实例间共享，节约内存
- **锁竞争**: 合理设计状态结构避免锁竞争
- **预热策略**: 在应用启动时预先加载常用数据

## 故障排除

### 常见错误

1. **downcast失败**: 检查状态类型是否正确匹配
2. **Send + Sync错误**: 确保状态中的所有字段都是线程安全的
3. **克隆错误**: 状态类型必须实现 `Clone`
4. **生命周期问题**: 确保状态的生命周期长于客户端

### 调试技巧

```rust
// 调试状态传递
if let Some(ctx) = context {
    println!("收到context，类型: {:?}", ctx.type_id());
    if let Some(state) = ctx.downcast_ref::<AppState>() {
        println!("成功获取AppState");
    } else {
        println!("downcast失败");
    }
} else {
    println!("未收到context");
}
```