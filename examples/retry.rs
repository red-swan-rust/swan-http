use std::time::Instant;
use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post, put, delete};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;
use log::{info, warn, error, debug};

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Serialize)]
struct CreateUserRequest {
    name: String,
    email: String,
}

/// 基础客户端 - 无重试
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct BasicClient;

impl BasicClient {
    #[get(url = "/users/1")]
    async fn get_user_no_retry(&self) -> anyhow::Result<User> {}
}

/// 重试客户端 - 展示各种重试策略
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct RetryClient;

impl RetryClient {
    /// GET 请求 - 使用指数退避重试（简化语法）
    #[get(url = "/users/1", retry = "exponential(3, 100ms)")]
    async fn get_user_with_retry(&self) -> anyhow::Result<User> {}
    
    /// GET 请求 - 使用完整配置的指数重试
    #[get(
        url = "/users/{user_id}", 
        retry = "exponential(max_attempts=5, base_delay=200ms, max_delay=10s, jitter_ratio=0.2)"
    )]
    async fn get_user_advanced_retry(&self, user_id: u32) -> anyhow::Result<User> {}
    
    /// POST 请求 - 固定延迟重试（非幂等，需要显式配置）
    #[post(
        url = "/users", 
        content_type = json,
        retry = "exponential(max_attempts=2, base_delay=500ms, idempotent_only=false)"
    )]
    async fn create_user_with_retry(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    /// PUT 请求 - 幂等操作，激进重试策略
    #[put(
        url = "/users/{user_id}",
        content_type = json,
        retry = "exponential(max_attempts=7, base_delay=50ms, exponential_base=1.5)"
    )]
    async fn update_user_aggressive_retry(&self, user_id: u32, body: CreateUserRequest) -> anyhow::Result<User> {}
    
    /// DELETE 请求 - 固定延迟重试
    #[delete(url = "/users/{user_id}", retry = "fixed(max_attempts=3, delay=1s)")]
    async fn delete_user_fixed_retry(&self, user_id: u32) -> anyhow::Result<User> {}
    
    /// 无重试的控制组
    #[get(url = "/users/{user_id}")]
    async fn get_user_no_retry(&self, user_id: u32) -> anyhow::Result<User> {}
}

/// 模拟不稳定的拦截器
#[derive(Default)]
struct UnstableInterceptor {
    failure_rate: f64,
}

impl UnstableInterceptor {
    pub fn new(failure_rate: f64) -> Self {
        Self { failure_rate }
    }
}

#[async_trait]
impl SwanInterceptor<()> for UnstableInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // 模拟网络不稳定
        if fastrand::f64() < self.failure_rate {
            return Err(anyhow::anyhow!("Simulated network instability"));
        }
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        // 模拟响应处理不稳定
        if fastrand::f64() < self.failure_rate * 0.5 {
            return Err(anyhow::anyhow!("Simulated response processing failure"));
        }
        Ok(response)
    }
}

/// 带不稳定拦截器的重试客户端
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    interceptor = UnstableInterceptor
)]
struct UnstableRetryClient;

impl UnstableRetryClient {
    /// 高重试容忍度的GET请求
    #[get(
        url = "/users/1", 
        retry = "exponential(max_attempts=8, base_delay=100ms, max_delay=5s)"
    )]
    async fn get_user_high_tolerance(&self) -> anyhow::Result<User> {}
}

async fn test_retry_strategy<F, Fut>(name: &str, operation: F, expected_success_rate: f64) 
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<User>>,
{
    println!("\n=== {} ===", name);
    
    let test_iterations = 5;
    let mut successful_calls = 0;
    let mut total_duration = std::time::Duration::ZERO;
    
    for i in 1..=test_iterations {
        let start = Instant::now();
        match operation().await {
            Ok(_) => {
                successful_calls += 1;
                let duration = start.elapsed();
                total_duration += duration;
                info!("  ✅ 测试 {}: 成功 ({}ms)", i, duration.as_millis());
            }
            Err(e) => {
                let duration = start.elapsed();
                total_duration += duration;
                error!("  ❌ 测试 {}: 失败 ({}ms) - {}", i, duration.as_millis(), e);
            }
        }
    }
    
    let success_rate = successful_calls as f64 / test_iterations as f64;
    let avg_duration = total_duration / test_iterations;
    
    info!("📊 结果: {}/{} 成功 ({:.1}%), 平均耗时: {}ms", 
             successful_calls, test_iterations, success_rate * 100.0, avg_duration.as_millis());
    
    if success_rate >= expected_success_rate {
        info!("✅ 达到预期成功率 {:.1}%", expected_success_rate * 100.0);
    } else {
        warn!("⚠️  未达到预期成功率 {:.1}%", expected_success_rate * 100.0);
    }
}

async fn retry_configuration_demo() {
    println!("=== 重试配置语法演示 ===");
    
    println!("\n支持的重试配置格式:");
    println!("1. 简化语法: retry = \"exponential(3, 100ms)\"");
    println!("   - max_attempts=3, base_delay=100ms, 默认指数底数2.0");
    
    println!("\n2. 完整配置: retry = \"exponential(max_attempts=5, base_delay=200ms, max_delay=10s, jitter_ratio=0.2)\"");
    println!("   - 最大重试5次, 基础延迟200ms, 最大延迟10秒, 20%随机抖动");
    
    println!("\n3. 固定延迟: retry = \"fixed(max_attempts=3, delay=1s)\"");
    println!("   - 固定间隔重试，不使用指数退避");
    
    println!("\n4. 非幂等支持: retry = \"exponential(..., idempotent_only=false)\"");
    println!("   - 允许POST请求重试（需要确保请求幂等性）");
    
    println!("\n重试触发条件:");
    println!("- HTTP 5xx 服务器错误");
    println!("- HTTP 429 限流");
    println!("- HTTP 408 请求超时");
    println!("- 网络连接错误");
    println!("- 请求超时");
}

async fn idempotent_safety_demo() {
    println!("\n=== 幂等性安全演示 ===");
    
    let retry_client = RetryClient::new();
    
    println!("✅ GET请求 - 天然幂等，支持重试");
    match retry_client.get_user_with_retry().await {
        Ok(_) => info!("  GET重试成功"),
        Err(e) => error!("  GET重试失败: {}", e),
    }
    
    println!("\n⚠️  POST请求 - 非幂等，默认禁止重试");
    println!("  需要显式设置 idempotent_only=false");
    
    println!("✅ PUT请求 - 幂等操作，支持重试");
    let create_req = CreateUserRequest {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    match retry_client.update_user_aggressive_retry(1, create_req).await {
        Ok(_) => info!("  PUT重试成功"),
        Err(e) => error!("  PUT重试失败: {}", e),
    }
    
    println!("✅ DELETE请求 - 幂等操作，支持重试");
    match retry_client.delete_user_fixed_retry(1).await {
        Ok(_) => info!("  DELETE重试成功"),
        Err(e) => error!("  DELETE重试失败: {}", e),
    }
}

async fn retry_timing_analysis() {
    println!("\n=== 重试时间分析 ===");
    
    // 模拟重试时间序列
    let base_delay = 100u64;
    let exponential_base = 2.0f64;
    let max_delay = 10000u64;
    let jitter_ratio = 0.1f64;
    
    println!("指数退避时间序列 (base={}ms, 指数={}, max={}ms):", 
             base_delay, exponential_base, max_delay);
    
    for attempt in 1..=6 {
        let exponential_delay = base_delay as f64 * exponential_base.powi((attempt - 1) as i32);
        let capped_delay = exponential_delay.min(max_delay as f64);
        let max_jitter = capped_delay * jitter_ratio;
        
        println!("  尝试 {}: ~{}ms (±{}ms jitter)", 
                 attempt, capped_delay as u64, max_jitter as u64);
    }
    
    println!("\n总重试时间估算:");
    let mut total_time = 0u64;
    for attempt in 1..=5 {
        let delay = (base_delay as f64 * exponential_base.powi((attempt - 1) as i32))
            .min(max_delay as f64) as u64;
        total_time += delay;
    }
    println!("  5次重试总延迟: ~{}ms ({:.1}s)", total_time, total_time as f64 / 1000.0);
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("=== Swan HTTP 渐进指数重试机制演示 ===");
    
    // 配置演示
    retry_configuration_demo().await;
    
    // 时间分析
    retry_timing_analysis().await;
    
    // 幂等性安全演示
    idempotent_safety_demo().await;
    
    println!("\n=== 实际重试测试 ===");
    
    let basic_client = BasicClient::new();
    let retry_client = RetryClient::new();
    
    // 测试无重试 vs 有重试的差异
    test_retry_strategy("基础客户端 (无重试)", || {
        basic_client.get_user_no_retry()
    }, 1.0).await;
    
    test_retry_strategy("重试客户端 (指数退避)", || {
        retry_client.get_user_with_retry()
    }, 1.0).await;
    
    test_retry_strategy("高级重试配置", || {
        retry_client.get_user_advanced_retry(1)
    }, 1.0).await;
    
    // 测试不稳定环境下的重试效果
    println!("\n=== 不稳定环境重试测试 ===");
    let unstable_client = UnstableRetryClient::new();
    
    test_retry_strategy("不稳定环境 + 高重试容忍", || {
        unstable_client.get_user_high_tolerance()
    }, 0.7).await; // 预期70%成功率
    
    println!("\n=== 重试机制特性总结 ===");
    println!("🎯 方法级配置: 每个API端点可以有独立的重试策略");
    println!("⚡ 指数退避: 避免对服务器造成额外压力");
    println!("🎲 随机抖动: 防止雷群效应，分散重试时间");
    println!("🔒 幂等性保护: 自动识别安全重试的HTTP方法");
    println!("📊 智能条件: 只对可恢复的错误进行重试");
    println!("🎛️  灵活配置: 支持多种重试策略和参数调优");
    
    Ok(())
}