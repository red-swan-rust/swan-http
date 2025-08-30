use serde::Deserialize;
use swan_macro::{http_client, get};
use std::time::Instant;
use tokio::time::Duration;

#[derive(Debug, Deserialize)]
struct Post {
    id: u32,
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: u32,
}

/// 集成测试客户端 - 测试不同重试策略
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct RetryIntegrationClient;

impl RetryIntegrationClient {
    /// 快速重试策略 - 适用于微服务场景
    #[get(url = "/posts/1", retry = "exponential(3, 50ms)")]
    async fn get_post_fast_retry(&self) -> anyhow::Result<Post> {}
    
    /// 温和重试策略 - 适用于外部API
    #[get(url = "/posts/2", retry = "exponential(max_attempts=5, base_delay=200ms, max_delay=10s, jitter_ratio=0.2)")]
    async fn get_post_gradual_retry(&self) -> anyhow::Result<Post> {}
    
    /// 固定延迟重试 - 适用于稳定服务
    #[get(url = "/posts/3", retry = "fixed(max_attempts=4, delay=500ms)")]
    async fn get_post_fixed_retry(&self) -> anyhow::Result<Post> {}
    
    /// 无重试对照组
    #[get(url = "/posts/4")]
    async fn get_post_no_retry(&self) -> anyhow::Result<Post> {}
}

/// 模拟服务器 - 用于测试重试行为
#[http_client(base_url = "http://httpbin.org")]
struct MockServerClient;

impl MockServerClient {
    /// 测试500错误重试
    #[get(url = "/status/500", retry = "exponential(3, 100ms)")]
    async fn test_500_error_retry(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试503错误重试
    #[get(url = "/status/503", retry = "exponential(3, 100ms)")]
    async fn test_503_error_retry(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试429限流重试
    #[get(url = "/status/429", retry = "exponential(max_attempts=3, base_delay=200ms, max_delay=5s)")]
    async fn test_rate_limit_retry(&self) -> anyhow::Result<serde_json::Value> {}
    
    /// 测试延迟响应
    #[get(url = "/delay/1", retry = "exponential(2, 100ms)")]
    async fn test_delay_tolerance(&self) -> anyhow::Result<serde_json::Value> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("=== Swan HTTP Retry Integration Tests ===\n");
    
    // 测试正常成功场景
    test_successful_requests().await?;
    
    // 测试重试时间测量
    test_retry_timing().await?;
    
    // 测试错误状态码重试
    test_error_status_retry().await;
    
    // 测试重试性能影响
    test_retry_performance_impact().await?;
    
    println!("\n✅ 所有集成测试完成");
    Ok(())
}

/// 测试成功请求场景
async fn test_successful_requests() -> anyhow::Result<()> {
    println!("📋 测试成功请求场景...");
    
    let client = RetryIntegrationClient::new();
    
    // 分别测试各种重试策略
    let test_scenarios = vec![
        ("快速重试", "fast"),
        ("温和重试", "gradual"),
        ("固定重试", "fixed"),
        ("无重试", "no_retry"),
    ];
    
    for (name, strategy_type) in test_scenarios {
        let start = Instant::now();
        let result = match strategy_type {
            "fast" => client.get_post_fast_retry().await,
            "gradual" => client.get_post_gradual_retry().await,
            "fixed" => client.get_post_fixed_retry().await,
            "no_retry" => client.get_post_no_retry().await,
            _ => unreachable!(),
        };
        
        match result {
            Ok(post) => {
                let duration = start.elapsed();
                println!("  ✅ {}: 成功获取文章 '{}' (耗时: {:?})", 
                        name, post.title.chars().take(30).collect::<String>(), duration);
            }
            Err(e) => {
                println!("  ❌ {}: 失败 - {}", name, e);
            }
        }
    }
    
    Ok(())
}

/// 测试重试时间测量
async fn test_retry_timing() -> anyhow::Result<()> {
    println!("\n⏱️  测试重试时间测量...");
    
    let client = RetryIntegrationClient::new();
    
    // 测试快速重试的时间特性
    let start = Instant::now();
    let _ = client.get_post_fast_retry().await;
    let duration = start.elapsed();
    
    println!("  📊 快速重试策略完成时间: {:?}", duration);
    
    // 成功的请求应该很快完成（不触发重试）
    if duration.as_millis() < 2000 {
        println!("  ✅ 时间性能符合预期（未触发重试）");
    } else {
        println!("  ⚠️  请求时间较长，可能网络环境影响");
    }
    
    Ok(())
}

/// 测试错误状态码重试行为
async fn test_error_status_retry() {
    println!("\n🔄 测试错误状态码重试行为...");
    
    let mock_client = MockServerClient::new();
    
    let error_tests = vec![
        ("500内部错误", "500"),
        ("503服务不可用", "503"),
        ("429限流", "429"),
    ];
    
    for (name, error_type) in error_tests {
        println!("  🧪 测试 {}...", name);
        let start = Instant::now();
        
        let result = match error_type {
            "500" => mock_client.test_500_error_retry().await,
            "503" => mock_client.test_503_error_retry().await,
            "429" => mock_client.test_rate_limit_retry().await,
            _ => unreachable!(),
        };
        
        match result {
            Ok(_) => {
                println!("    ✅ 意外成功（可能服务器行为已改变）");
            }
            Err(e) => {
                let duration = start.elapsed();
                println!("    ❌ 预期失败: {} (总耗时: {:?})", e, duration);
                
                // 验证重试是否实际发生（通过时间判断）
                if duration.as_millis() > 300 {
                    println!("    🔄 检测到重试行为（基于耗时判断）");
                } else {
                    println!("    ⚡ 快速失败（可能未触发重试）");
                }
            }
        }
    }
}

/// 测试重试对性能的影响
async fn test_retry_performance_impact() -> anyhow::Result<()> {
    println!("\n📈 测试重试性能影响...");
    
    let _client = RetryIntegrationClient::new();
    
    // 并发测试不同策略
    let concurrent_requests = 5;
    let mut handles = Vec::new();
    
    for i in 0..concurrent_requests {
        let client = RetryIntegrationClient::new();
        let handle = tokio::spawn(async move {
            let start = Instant::now();
            let result = client.get_post_fast_retry().await;
            let duration = start.elapsed();
            (i, result.is_ok(), duration)
        });
        handles.push(handle);
    }
    
    let mut total_duration = Duration::ZERO;
    let mut success_count = 0;
    
    for handle in handles {
        if let Ok((id, success, duration)) = handle.await {
            total_duration += duration;
            if success {
                success_count += 1;
            }
            println!("  📊 并发请求 #{}: {} (耗时: {:?})", 
                    id, if success { "成功" } else { "失败" }, duration);
        }
    }
    
    let avg_duration = total_duration / concurrent_requests;
    println!("  📈 平均响应时间: {:?}", avg_duration);
    println!("  📊 成功率: {}/{} ({:.1}%)", 
            success_count, concurrent_requests, 
            (success_count as f64 / concurrent_requests as f64) * 100.0);
    
    Ok(())
}

/// 重试策略比较测试
#[cfg(feature = "extended_tests")]
async fn test_retry_strategy_comparison() -> anyhow::Result<()> {
    println!("\n🔬 重试策略比较测试...");
    
    let client = RetryIntegrationClient::new();
    
    // 比较不同策略的表现
    let strategies = vec![
        ("指数重试", "fast"),
        ("温和重试", "gradual"),
        ("固定重试", "fixed"),
        ("无重试", "no_retry"),
    ];
    
    for (strategy_name, strategy_type) in strategies {
        let start = Instant::now();
        let result = match strategy_type {
            "fast" => client.get_post_fast_retry().await,
            "gradual" => client.get_post_gradual_retry().await,
            "fixed" => client.get_post_fixed_retry().await,
            "no_retry" => client.get_post_no_retry().await,
            _ => unreachable!(),
        };
        let duration = start.elapsed();
        
        println!("  📊 {}: {} (耗时: {:?})", 
                strategy_name,
                if result.is_ok() { "成功" } else { "失败" },
                duration);
    }
    
    Ok(())
}

/// 网络条件模拟测试
#[cfg(feature = "network_simulation")]
async fn test_network_conditions() -> anyhow::Result<()> {
    println!("\n🌐 网络条件模拟测试...");
    
    let mock_client = MockServerClient::new();
    
    // 测试延迟容忍度
    println!("  🐌 测试高延迟场景...");
    let start = Instant::now();
    match mock_client.test_delay_tolerance().await {
        Ok(_) => {
            let duration = start.elapsed();
            println!("    ✅ 延迟容忍测试成功 (耗时: {:?})", duration);
        }
        Err(e) => {
            println!("    ❌ 延迟容忍测试失败: {}", e);
        }
    }
    
    Ok(())
}