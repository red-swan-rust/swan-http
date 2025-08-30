# Swan HTTP 重试机制

## 概述

Swan HTTP 提供了强大而灵活的方法级重试机制，支持渐进式指数退避算法，帮助处理网络不稳定和服务暂时不可用的情况。

## 核心特性

- **方法级配置**: 在每个HTTP方法上独立配置重试策略
- **指数退避算法**: 智能的延迟增长，避免服务器过载
- **随机抖动**: 防止雷群效应，分散重试时间
- **幂等性保护**: 自动检测HTTP方法幂等性，确保安全重试
- **智能重试条件**: 基于HTTP状态码的智能重试判断
- **高性能**: 编译时优化，运行时零额外开销

## 基础用法

### 简单重试配置

```rust
use swan_macro::{http_client, get};

#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    /// 使用指数重试：最多3次，基础延迟100ms
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    /// 使用固定延迟重试：最多5次，每次延迟500ms
    #[get(url = "/posts", retry = "fixed(max_attempts=5, delay=500ms)")]
    async fn get_posts(&self) -> anyhow::Result<Vec<Post>> {}
}
```

## 重试策略

### 指数退避重试 (Exponential)

指数退避是推荐的重试策略，延迟时间按指数增长，适合大多数场景。

```rust
// 基础格式
#[get(url = "/api/data", retry = "exponential(3, 100ms)")]

// 完整配置格式
#[get(url = "/api/data", retry = "exponential(
    max_attempts=5,
    base_delay=200ms,
    max_delay=30s,
    exponential_base=2.0,
    jitter_ratio=0.1,
    idempotent_only=true
)")]
```

**参数说明:**
- `max_attempts`: 最大重试次数（包含首次请求）
- `base_delay`: 基础延迟时间，支持 `ms`(毫秒) 和 `s`(秒)
- `max_delay`: 最大延迟时间，防止延迟过长
- `exponential_base`: 指数底数，控制增长速度（默认2.0）
- `jitter_ratio`: 随机抖动比例，0.0-1.0（默认0.1）
- `idempotent_only`: 是否仅对幂等方法重试（默认true）

**延迟计算公式:**
```
delay = min(base_delay * exponential_base^(attempt-1) + jitter, max_delay)
```

### 固定延迟重试 (Fixed)

固定延迟重试在每次重试时使用相同的延迟时间，适合稳定的服务环境。

```rust
// 基础格式
#[get(url = "/api/data", retry = "fixed(max_attempts=3, delay=1s)")]
```

**参数说明:**
- `max_attempts`: 最大重试次数
- `delay`: 固定延迟时间

## 重试条件

### 自动重试的状态码

- **5xx 服务器错误** (500-599): 服务器内部错误，通常是临时的
- **429 Too Many Requests**: 限流，服务器过载
- **408 Request Timeout**: 请求超时

### 不会重试的状态码

- **2xx 成功响应**: 请求成功
- **4xx 客户端错误** (除408, 429): 客户端请求问题，重试无意义

### 网络错误

所有网络连接错误（如连接超时、DNS解析失败等）都会触发重试。

## 幂等性保护

### 什么是幂等性？

幂等操作是指多次执行产生相同结果的操作。在HTTP中：

- **幂等方法**: GET, PUT, DELETE
- **非幂等方法**: POST

### 安全重试

默认情况下，只有幂等方法会自动重试：

```rust
impl ApiClient {
    /// GET方法：自动重试 ✅
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    /// POST方法：默认不重试 ⚠️
    #[post(url = "/users", retry = "exponential(3, 100ms)")]  // 不会实际重试
    async fn create_user(&self, user: User) -> anyhow::Result<User> {}
    
    /// POST方法：强制重试 ⚠️ （谨慎使用）
    #[post(url = "/idempotent-action", retry = "exponential(
        max_attempts=3, 
        base_delay=100ms, 
        idempotent_only=false
    )")]
    async fn safe_post_action(&self, data: Data) -> anyhow::Result<Response> {}
}
```

## 配置示例

### 微服务场景

快速重试，适用于内部服务调用：

```rust
#[get(url = "/internal/service", retry = "exponential(3, 50ms)")]
async fn call_internal_service(&self) -> anyhow::Result<ServiceResponse> {}
```

### 外部API场景

温和重试，考虑外部服务的负载：

```rust
#[get(url = "/external/api", retry = "exponential(
    max_attempts=5,
    base_delay=500ms,
    max_delay=30s,
    exponential_base=1.5,
    jitter_ratio=0.3
)")]
async fn call_external_api(&self) -> anyhow::Result<ExternalData> {}
```

### 限流敏感场景

较长的延迟和温和增长，应对限流：

```rust
#[get(url = "/rate-limited-api", retry = "exponential(
    max_attempts=7,
    base_delay=1s,
    max_delay=60s,
    exponential_base=1.2,
    jitter_ratio=0.5
)")]
async fn call_rate_limited_api(&self) -> anyhow::Result<Data> {}
```

### 稳定服务场景

固定延迟，可预测的重试时间：

```rust
#[get(url = "/stable/service", retry = "fixed(max_attempts=4, delay=1s)")]
async fn call_stable_service(&self) -> anyhow::Result<Data> {}
```

## 最佳实践

### 1. 选择合适的重试策略

- **微服务内部调用**: 使用快速指数重试 `exponential(3, 50ms)`
- **外部API调用**: 使用温和重试 `exponential(5, 500ms)`
- **限流敏感**: 使用长延迟和大抖动 `exponential(7, 1s, jitter_ratio=0.5)`
- **可预测场景**: 使用固定延迟 `fixed(3, 1s)`

### 2. 合理设置参数

```rust
// ✅ 好的配置
#[get(url = "/api", retry = "exponential(
    max_attempts=3,      // 适中的重试次数
    base_delay=100ms,    // 合理的基础延迟
    max_delay=10s,       // 防止延迟过长
    jitter_ratio=0.1     // 适度的抖动
)")]

// ❌ 不好的配置
#[get(url = "/api", retry = "exponential(
    max_attempts=50,     // 过多的重试次数
    base_delay=1ms,      // 过短的延迟，可能造成雷群
    max_delay=3600s,     // 过长的最大延迟
    jitter_ratio=1.0     // 过大的抖动比例
)")]
```

### 3. 注意幂等性

```rust
// ✅ 安全的重试
#[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
#[put(url = "/users/{id}", retry = "exponential(3, 100ms)")]
#[delete(url = "/users/{id}", retry = "exponential(3, 100ms)")]

// ⚠️ 谨慎使用
#[post(url = "/orders", retry = "exponential(
    max_attempts=3,
    base_delay=100ms,
    idempotent_only=false  // 显式允许非幂等重试
)")]
```

### 4. 监控和调试

在开发环境启用调试日志：

```rust
// 在main函数中
env_logger::init();

// 或者更详细的配置
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
```

日志输出示例：
```
WARN: Request failed with status 503, retrying attempt 2/3
DEBUG: Retry attempt 2/3 after 200ms delay. Reason: Service Unavailable
```

## 错误处理

### 重试失败后的错误

当所有重试都失败后，会返回最后一次的错误：

```rust
match client.get_data_with_retry().await {
    Ok(data) => println!("成功: {:?}", data),
    Err(e) => {
        // e 包含最后一次重试的错误信息
        eprintln!("重试失败: {}", e);
    }
}
```

### 非幂等方法的重试错误

当尝试重试非幂等方法且 `idempotent_only=true` 时：

```rust
// POST方法默认不会实际重试，即使配置了retry参数
#[post(url = "/users", retry = "exponential(3, 100ms)")]
async fn create_user(&self, user: User) -> anyhow::Result<User> {}
```

## 性能考虑

### 内存占用

`RetryPolicy` 结构体经过优化，内存占用 ≤ 64 bytes，适合高频使用。

### 计算性能

延迟计算算法高度优化：
- 1000次延迟计算 < 10ms
- 100次配置解析 < 100ms

### 并发安全

重试机制完全线程安全，支持高并发场景。

## 故障排除

### 常见问题

1. **重试没有生效**
   - 检查HTTP方法是否幂等（GET/PUT/DELETE）
   - 确认 `idempotent_only` 设置
   - 验证状态码是否在重试范围内

2. **重试时间过长**
   - 减少 `max_attempts`
   - 降低 `exponential_base`
   - 设置合理的 `max_delay`

3. **配置解析错误**
   - 检查语法格式是否正确
   - 确认时间单位（ms/s）
   - 验证参数名拼写

### 调试技巧

```rust
// 启用详细日志
RUST_LOG=debug cargo run --example retry_integration_test

// 测试特定重试配置
#[get(url = "/test", retry = "exponential(
    max_attempts=2,    // 减少重试次数便于观察
    base_delay=1s,     // 增加延迟便于观察
    jitter_ratio=0.0   // 无抖动，时间可预测
)")]
```

## 高级用法

### 自定义重试条件

虽然默认重试条件覆盖了大多数场景，但可以通过组合不同的配置来实现特殊需求：

```rust
// 激进重试：更多次数，更快增长
#[get(url = "/critical-service", retry = "exponential(
    max_attempts=10,
    base_delay=10ms,
    max_delay=5s,
    exponential_base=3.0
)")]

// 保守重试：较少次数，温和增长
#[get(url = "/unstable-service", retry = "exponential(
    max_attempts=3,
    base_delay=2s,
    max_delay=30s,
    exponential_base=1.2
)")]
```

### 场景化配置

```rust
impl ApiClient {
    // 🔥 高频微服务调用
    #[get(url = "/internal/health", retry = "exponential(3, 25ms)")]
    async fn health_check(&self) -> anyhow::Result<HealthStatus> {}
    
    // 🌐 第三方API集成
    #[get(url = "/external/weather", retry = "exponential(
        max_attempts=5,
        base_delay=1s,
        max_delay=60s,
        jitter_ratio=0.3
    )")]
    async fn get_weather(&self, city: String) -> anyhow::Result<Weather> {}
    
    // 📊 数据分析服务（可能处理时间长）
    #[get(url = "/analytics/report", retry = "exponential(
        max_attempts=7,
        base_delay=2s,
        max_delay=300s,
        exponential_base=1.5
    )")]
    async fn generate_report(&self, params: ReportParams) -> anyhow::Result<Report> {}
}