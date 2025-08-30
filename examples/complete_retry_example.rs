use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post, put, delete};
use std::time::Instant;
use async_trait::async_trait;
use swan_common::SwanInterceptor;
use std::borrow::Cow;
use std::any::Any;

#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

#[derive(Debug, Deserialize)]
struct Post {
    id: u32,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: u32,
}

#[derive(Serialize, Clone)]
struct CreatePostRequest {
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: u32,
}

/// 重试监控拦截器 - 记录重试行为
#[derive(Default)]
struct RetryMonitoringInterceptor;

#[async_trait]
impl SwanInterceptor for RetryMonitoringInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        log::info!("🚀 发送请求到: {}", request.try_clone().unwrap().build().unwrap().url());
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        let status = response.status();
        if status.is_success() {
            log::info!("✅ 响应成功: {}", status);
        } else {
            log::warn!("⚠️ 响应错误: {} - {}", status, status.canonical_reason().unwrap_or("未知错误"));
        }
        Ok(response)
    }
}

/// 完整重试示例客户端
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    interceptor = RetryMonitoringInterceptor
)]
struct CompleteRetryExampleClient;

impl CompleteRetryExampleClient {
    // ========== 指数重试策略示例 ==========
    
    /// 快速重试 - 微服务内部调用
    #[get(url = "/users/{id}", retry = "exponential(3, 50ms)")]
    async fn get_user_fast_retry(&self, id: u32) -> anyhow::Result<User> {}
    
    /// 标准重试 - 一般外部API
    #[get(url = "/posts/{id}", retry = "exponential(5, 200ms)")]
    async fn get_post_standard_retry(&self, id: u32) -> anyhow::Result<Post> {}
    
    /// 温和重试 - 不稳定的外部服务
    #[get(url = "/users", retry = "exponential(
        max_attempts=7,
        base_delay=500ms,
        max_delay=60s,
        exponential_base=1.5,
        jitter_ratio=0.3
    )")]
    async fn get_users_gentle_retry(&self) -> anyhow::Result<Vec<User>> {}
    
    /// 激进重试 - 关键业务接口
    #[get(url = "/posts", retry = "exponential(
        max_attempts=10,
        base_delay=100ms,
        max_delay=30s,
        exponential_base=2.5,
        jitter_ratio=0.2
    )")]
    async fn get_posts_aggressive_retry(&self) -> anyhow::Result<Vec<Post>> {}
    
    // ========== 固定延迟重试示例 ==========
    
    /// 固定延迟 - 稳定服务
    #[get(url = "/users/1", retry = "fixed(max_attempts=4, delay=1s)")]
    async fn get_user_fixed_retry(&self) -> anyhow::Result<User> {}
    
    /// 短固定延迟 - 本地服务
    #[get(url = "/posts/1", retry = "fixed(max_attempts=3, delay=100ms)")]
    async fn get_post_short_fixed(&self) -> anyhow::Result<Post> {}
    
    // ========== 不同HTTP方法的重试行为 ==========
    
    /// GET：默认会重试（幂等安全）
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user_safe_retry(&self, id: u32) -> anyhow::Result<User> {}
    
    /// PUT：默认会重试（幂等安全）
    #[put(url = "/users/{id}", content_type = json, retry = "exponential(3, 100ms)")]
    async fn update_user_safe_retry(&self, id: u32, body: User) -> anyhow::Result<User> {}
    
    /// DELETE：默认会重试（幂等安全）
    #[delete(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn delete_user_safe_retry(&self, id: u32) -> anyhow::Result<()> {}
    
    /// POST：默认不重试（非幂等）
    #[post(url = "/posts", content_type = json, retry = "exponential(3, 100ms)")]
    async fn create_post_no_retry(&self, body: CreatePostRequest) -> anyhow::Result<Post> {}
    
    /// POST：强制重试（需要确保操作幂等）
    #[post(url = "/idempotent-posts", content_type = json, retry = "exponential(
        max_attempts=3,
        base_delay=100ms,
        idempotent_only=false
    )")]
    async fn create_idempotent_post(&self, body: CreatePostRequest) -> anyhow::Result<Post> {}
    
    // ========== 无重试对照组 ==========
    
    /// 无重试版本 - 用于对比
    #[get(url = "/users/{id}")]
    async fn get_user_no_retry(&self, id: u32) -> anyhow::Result<User> {}
}

/// 错误场景测试客户端
#[http_client(base_url = "http://httpbin.org")]
struct ErrorTestClient;

impl ErrorTestClient {
    /// 测试500错误重试
    #[get(url = "/status/500", retry = "exponential(3, 200ms)")]
    async fn test_500_error(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试503服务不可用重试
    #[get(url = "/status/503", retry = "exponential(4, 300ms)")]
    async fn test_503_error(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试429限流重试
    #[get(url = "/status/429", retry = "exponential(
        max_attempts=5,
        base_delay=1s,
        max_delay=30s,
        jitter_ratio=0.4
    )")]
    async fn test_429_rate_limit(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试408超时重试
    #[get(url = "/status/408", retry = "exponential(3, 500ms)")]
    async fn test_408_timeout(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试404不重试（正确行为）
    #[get(url = "/status/404", retry = "exponential(3, 100ms)")]
    async fn test_404_no_retry(&self) -> anyhow::Result<serde_json::Value> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志以观察重试行为
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    
    println!("=== Swan HTTP 完整重试示例 ===\n");
    
    // 演示各种重试策略
    demo_retry_strategies().await?;
    
    // 演示HTTP方法的重试行为
    demo_http_method_retry_behavior().await?;
    
    // 演示错误场景重试
    demo_error_scenario_retry().await;
    
    // 性能比较测试
    demo_performance_comparison().await?;
    
    // 显示推荐指南
    print_retry_strategy_guide_async().await;
    
    println!("\n🎉 完整重试示例演示结束");
    Ok(())
}

/// 演示各种重试策略
async fn demo_retry_strategies() -> anyhow::Result<()> {
    println!("📋 演示重试策略...\n");
    
    let client = CompleteRetryExampleClient::new();
    
    // 分别测试不同策略
    let test_cases = vec![
        ("快速重试 (微服务)", "user", 1),
        ("标准重试 (外部API)", "post", 1),
        ("温和重试 (不稳定服务)", "users", 0),
        ("激进重试 (关键业务)", "posts", 0),
        ("固定延迟 (稳定服务)", "user_fixed", 0),
        ("短固定延迟 (本地服务)", "post_fixed", 0),
    ];
    
    for (name, test_type, param) in test_cases {
        println!("🧪 测试 {}...", name);
        let start = Instant::now();
        
        let result = match test_type {
            "user" => client.get_user_fast_retry(param).await.map(|_| ()),
            "post" => client.get_post_standard_retry(param).await.map(|_| ()),
            "users" => client.get_users_gentle_retry().await.map(|_| ()),
            "posts" => client.get_posts_aggressive_retry().await.map(|_| ()),
            "user_fixed" => client.get_user_fixed_retry().await.map(|_| ()),
            "post_fixed" => client.get_post_short_fixed().await.map(|_| ()),
            _ => unreachable!(),
        };
        
        match result {
            Ok(_) => {
                let duration = start.elapsed();
                println!("  ✅ 成功 (耗时: {:?})\n", duration);
            }
            Err(e) => {
                let duration = start.elapsed();
                println!("  ❌ 失败: {} (耗时: {:?})\n", e, duration);
            }
        }
    }
    
    Ok(())
}

/// 演示不同HTTP方法的重试行为
async fn demo_http_method_retry_behavior() -> anyhow::Result<()> {
    println!("🔄 演示HTTP方法重试行为...\n");
    
    let client = CompleteRetryExampleClient::new();
    
    // 测试幂等方法（会重试）
    println!("📗 幂等方法测试（会自动重试）:");
    
    let get_start = Instant::now();
    match client.get_user_safe_retry(1).await {
        Ok(user) => println!("  ✅ GET成功: {} (耗时: {:?})", user.name, get_start.elapsed()),
        Err(e) => println!("  ❌ GET失败: {} (耗时: {:?})", e, get_start.elapsed()),
    }
    
    let update_user = User { id: 1, name: "Updated Name".to_string(), email: "updated@example.com".to_string() };
    let put_start = Instant::now();
    match client.update_user_safe_retry(1, update_user).await {
        Ok(_) => println!("  ✅ PUT成功 (耗时: {:?})", put_start.elapsed()),
        Err(e) => println!("  ❌ PUT失败: {} (耗时: {:?})", e, put_start.elapsed()),
    }
    
    let delete_start = Instant::now();
    match client.delete_user_safe_retry(999).await {
        Ok(_) => println!("  ✅ DELETE成功 (耗时: {:?})", delete_start.elapsed()),
        Err(e) => println!("  ❌ DELETE失败: {} (耗时: {:?})", e, delete_start.elapsed()),
    }
    
    // 测试非幂等方法（默认不重试）
    println!("\n📕 非幂等方法测试:");
    
    let create_post = CreatePostRequest {
        title: "Test Post".to_string(),
        body: "This is a test post body".to_string(),
        user_id: 1,
    };
    
    let post_start = Instant::now();
    match client.create_post_no_retry(create_post.clone()).await {
        Ok(_) => println!("  ✅ POST成功（默认无重试） (耗时: {:?})", post_start.elapsed()),
        Err(e) => println!("  ❌ POST失败（默认无重试）: {} (耗时: {:?})", e, post_start.elapsed()),
    }
    
    let idempotent_post_start = Instant::now();
    match client.create_idempotent_post(create_post).await {
        Ok(_) => println!("  ✅ POST成功（强制重试） (耗时: {:?})", idempotent_post_start.elapsed()),
        Err(e) => println!("  ❌ POST失败（强制重试）: {} (耗时: {:?})", e, idempotent_post_start.elapsed()),
    }
    
    println!();
    Ok(())
}

/// 演示错误场景重试
async fn demo_error_scenario_retry() {
    println!("💥 演示错误场景重试...\n");
    
    let error_client = ErrorTestClient::new();
    
    let error_scenarios = vec![
        ("500 内部服务器错误", "500"),
        ("503 服务不可用", "503"),
        ("429 限流", "429"),
        ("408 请求超时", "408"),
        ("404 未找到（不重试）", "404"),
    ];
    
    for (scenario_name, status_code) in error_scenarios {
        println!("🧪 测试场景: {}", scenario_name);
        let start = Instant::now();
        
        let result = match status_code {
            "500" => error_client.test_500_error().await,
            "503" => error_client.test_503_error().await,
            "429" => error_client.test_429_rate_limit().await,
            "408" => error_client.test_408_timeout().await,
            "404" => error_client.test_404_no_retry().await,
            _ => unreachable!(),
        };
        
        match result {
            Ok(_) => {
                println!("  🎉 意外成功！（服务器可能已修复）");
            }
            Err(e) => {
                let duration = start.elapsed();
                println!("  ❌ 预期失败: {}", e);
                
                // 通过执行时间判断是否发生了重试
                if duration.as_millis() > 200 {
                    println!("  🔄 检测到重试行为（总耗时: {:?}）", duration);
                } else {
                    println!("  ⚡ 快速失败（可能未重试，耗时: {:?}）", duration);
                }
            }
        }
        println!();
    }
}

/// 性能比较测试
async fn demo_performance_comparison() -> anyhow::Result<()> {
    println!("📊 性能比较测试...\n");
    
    let client = CompleteRetryExampleClient::new();
    
    // 比较有重试和无重试的性能差异
    println!("🏃‍♂️ 成功请求性能对比:");
    
    // 无重试版本
    let start = Instant::now();
    match client.get_user_no_retry(1).await {
        Ok(_) => {
            let duration = start.elapsed();
            println!("  📈 无重试: {:?}", duration);
        }
        Err(e) => println!("  ❌ 无重试失败: {}", e),
    }
    
    // 有重试版本（成功场景）
    let start = Instant::now();
    match client.get_user_fast_retry(1).await {
        Ok(_) => {
            let duration = start.elapsed();
            println!("  📈 快速重试: {:?}", duration);
        }
        Err(e) => println!("  ❌ 快速重试失败: {}", e),
    }
    
    // 并发性能测试
    println!("\n🚀 并发重试性能测试:");
    let concurrent_count = 5;
    let mut handles = Vec::new();
    
    let start_all = Instant::now();
    for i in 0..concurrent_count {
        let client = CompleteRetryExampleClient::new();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = client.get_user_fast_retry(i + 1).await;
            (i, result.is_ok(), start.elapsed())
        });
        handles.push(handle);
    }
    
    let mut success_count = 0;
    for handle in handles {
        if let Ok((id, success, duration)) = handle.await {
            if success {
                success_count += 1;
                println!("  ✅ 请求#{}: 成功 ({:?})", id, duration);
            } else {
                println!("  ❌ 请求#{}: 失败 ({:?})", id, duration);
            }
        }
    }
    
    let total_duration = start_all.elapsed();
    println!("  📊 总体结果: {}/{} 成功, 总耗时: {:?}", success_count, concurrent_count, total_duration);
    println!("  📈 平均每请求: {:?}", total_duration / concurrent_count);
    
    Ok(())
}

/// 重试策略推荐指南
fn print_retry_strategy_guide() {
    println!("\n📚 重试策略选择指南:\n");
    
    println!("🏗️ **微服务内部调用**");
    println!("   推荐: exponential(3, 50ms)");
    println!("   特点: 快速重试，短延迟，适合可靠网络\n");
    
    println!("🌐 **外部API调用**");
    println!("   推荐: exponential(5, 200ms)");
    println!("   特点: 适中重试，考虑网络延迟\n");
    
    println!("🔒 **限流敏感服务**");
    println!("   推荐: exponential(max_attempts=7, base_delay=1s, jitter_ratio=0.5)");
    println!("   特点: 温和重试，大抖动，避免雷群\n");
    
    println!("⚡ **关键业务接口**");
    println!("   推荐: exponential(max_attempts=10, base_delay=100ms, exponential_base=2.5)");
    println!("   特点: 激进重试，确保成功\n");
    
    println!("🛠️ **稳定内部服务**");
    println!("   推荐: fixed(max_attempts=4, delay=500ms)");
    println!("   特点: 可预测延迟，简单可靠\n");
    
    println!("💡 **最佳实践提示:**");
    println!("   • GET/PUT/DELETE 方法默认会重试（幂等安全）");
    println!("   • POST 方法默认不重试（防止重复提交）");
    println!("   • 使用 idempotent_only=false 可强制重试POST（谨慎使用）");
    println!("   • 设置合理的 max_delay 防止重试时间过长");
    println!("   • 适当的 jitter_ratio 可以避免雷群效应");
}

/// 重试策略推荐指南
async fn print_retry_strategy_guide_async() {
    print_retry_strategy_guide();
    
    println!("\n🧪 **实际测试建议:**");
    println!("   cargo run --example simple_retry_test      # 基础功能验证");
    println!("   cargo run --example retry_integration_test # 完整集成测试");
    println!("   RUST_LOG=debug cargo run --example complete_retry_example # 详细重试日志");
}