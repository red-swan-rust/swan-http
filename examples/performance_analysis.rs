use std::time::Instant;
use serde::Deserialize;
use swan_macro::{http_client, get};
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;
use log::{info, warn, error, debug};

#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 无拦截器的基础客户端
#[http_client(base_url = "https://jsonplaceholder.typicode.com")]
struct BasicClient;

impl BasicClient {
    #[get(url = "/users/1")]
    async fn get_user_static(&self) -> anyhow::Result<User> {}
    
    #[get(url = "/users/{user_id}")]
    async fn get_user_dynamic(&self, user_id: u32) -> anyhow::Result<User> {}
}

/// 空拦截器（测试拦截器开销）
#[derive(Default)]
struct NoOpInterceptor;

#[async_trait]
impl SwanInterceptor for NoOpInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

/// 带拦截器的客户端
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com", 
    interceptor = NoOpInterceptor
)]
struct InterceptorClient;

impl InterceptorClient {
    #[get(url = "/users/1")]
    async fn get_user_static(&self) -> anyhow::Result<User> {}
    
    #[get(url = "/users/{user_id}")]
    async fn get_user_dynamic(&self, user_id: u32) -> anyhow::Result<User> {}
}

/// 应用状态
#[derive(Clone)]
struct AppState {
    counter: std::sync::Arc<std::sync::RwLock<u64>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            counter: std::sync::Arc::new(std::sync::RwLock::new(0)),
        }
    }
}

/// 状态感知拦截器
#[derive(Default)]
struct StateAwareInterceptor;

#[async_trait]
impl SwanStatefulInterceptor<AppState> for StateAwareInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        state: Option<&AppState>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        if let Some(_app_state) = state {
            debug!("📊 状态感知拦截器: 访问AppState");
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

/// 带状态的客户端
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    interceptor = StateAwareInterceptor,
    state = AppState
)]
struct StatefulClient;

impl StatefulClient {
    #[get(url = "/users/1")]
    async fn get_user_static(&self) -> anyhow::Result<User> {}
    
    #[get(url = "/users/{user_id}")]
    async fn get_user_dynamic(&self, user_id: u32) -> anyhow::Result<User> {}
}

async fn benchmark_client<F, Fut>(name: &str, operation: F) -> u128 
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<User>>,
{
    let start = Instant::now();
    let iterations = 1; // 单次测试，避免网络影响
    
    for _ in 0..iterations {
        let _ = operation().await;
    }
    
    let duration = start.elapsed().as_nanos() / iterations as u128;
    info!("{}: {}ns per call", name, duration);
    duration
}

fn memory_footprint_analysis() {
    println!("\n=== 内存占用分析 ===");
    
    let basic_client = BasicClient::new();
    let interceptor_client = InterceptorClient::new();
    let app_state = AppState::new();
    let stateful_client = StatefulClient::new().with_state(app_state);
    
    println!("1. 基础客户端: {:?} 字节", std::mem::size_of_val(&basic_client));
    println!("2. 拦截器客户端: {:?} 字节", std::mem::size_of_val(&interceptor_client));
    println!("3. 状态客户端: {:?} 字节", std::mem::size_of_val(&stateful_client));
    
    println!("\n各组件大小:");
    println!("- reqwest::Client: {:?} 字节", std::mem::size_of::<reqwest::Client>());
    println!("- String (base_url): {:?} 字节", std::mem::size_of::<String>());
    println!("- Option<Arc<dyn SwanInterceptor>>: {:?} 字节", 
             std::mem::size_of::<Option<std::sync::Arc<dyn SwanInterceptor + Send + Sync>>>());
    println!("- Mutex<InterceptorCache>: {:?} 字节", 
             std::mem::size_of::<std::sync::Mutex<swan_common::InterceptorCache>>());
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Swan HTTP 性能分析 ===\n");
    
    // 内存占用分析
    memory_footprint_analysis();
    
    println!("\n=== 运行时性能对比 ===");
    
    // 创建客户端实例
    let basic_client = BasicClient::new();
    let interceptor_client = InterceptorClient::new();
    let app_state = AppState::new();
    let stateful_client = StatefulClient::new().with_state(app_state);
    
    println!("测试目标: jsonplaceholder.typicode.com/users/1\n");
    
    // 性能基准测试
    let basic_static = benchmark_client("基础客户端 (静态URL)", || {
        basic_client.get_user_static()
    }).await;
    
    let basic_dynamic = benchmark_client("基础客户端 (动态参数)", || {
        basic_client.get_user_dynamic(1)
    }).await;
    
    let interceptor_static = benchmark_client("拦截器客户端 (静态URL)", || {
        interceptor_client.get_user_static()
    }).await;
    
    let interceptor_dynamic = benchmark_client("拦截器客户端 (动态参数)", || {
        interceptor_client.get_user_dynamic(1)
    }).await;
    
    let stateful_static = benchmark_client("状态客户端 (静态URL)", || {
        stateful_client.get_user_static()
    }).await;
    
    let stateful_dynamic = benchmark_client("状态客户端 (动态参数)", || {
        stateful_client.get_user_dynamic(1)
    }).await;
    
    println!("\n=== 性能分析结论 ===");
    
    // 动态参数开销分析
    let dynamic_overhead = if basic_static > 0 {
        ((basic_dynamic as f64 - basic_static as f64) / basic_static as f64) * 100.0
    } else { 0.0 };
    
    // 拦截器开销分析  
    let interceptor_overhead = if basic_static > 0 {
        ((interceptor_static as f64 - basic_static as f64) / basic_static as f64) * 100.0
    } else { 0.0 };
    
    // 状态注入开销分析
    let state_overhead = if basic_static > 0 {
        ((stateful_static as f64 - basic_static as f64) / basic_static as f64) * 100.0
    } else { 0.0 };
    
    println!("1. 动态参数开销: {:.2}%", dynamic_overhead);
    println!("2. 拦截器开销: {:.2}%", interceptor_overhead);
    println!("3. 状态注入开销: {:.2}%", state_overhead);
    
    println!("\n💡 性能特征:");
    println!("- 网络延迟 >> 框架开销 (通常网络请求需要几十ms)");
    println!("- 编译时优化确保运行时高效");
    println!("- 零拷贝设计减少内存分配");
    println!("- 拦截器缓存避免重复创建");
    
    Ok(())
}