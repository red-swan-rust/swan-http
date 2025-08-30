use serde::Deserialize;
use swan_macro::{http_client, get};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::any::Any;

/// 应用状态结构
/// 
/// 模拟真实应用中的状态管理，包含缓存、配置等
#[derive(Clone)]
struct AppState {
    // 模拟Redis缓存
    cache: Arc<RwLock<HashMap<String, String>>>,
    // 模拟配置
    config: Arc<RwLock<HashMap<String, String>>>,
    // 请求计数器
    request_counter: Arc<RwLock<u64>>,
}

impl AppState {
    pub fn new() -> Self {
        let mut cache = HashMap::new();
        cache.insert("auth_token".to_string(), "cached-jwt-token-12345".to_string());
        cache.insert("user_id".to_string(), "user_001".to_string());
        
        let mut config = HashMap::new();
        config.insert("api_version".to_string(), "v1".to_string());
        config.insert("client_id".to_string(), "swan-http-client".to_string());
        
        Self {
            cache: Arc::new(RwLock::new(cache)),
            config: Arc::new(RwLock::new(config)),
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

/// 支持状态的认证拦截器
/// 
/// 演示如何从应用状态中获取认证信息
struct StatefulAuthInterceptor {
    state: Option<AppState>,
}

impl Default for StatefulAuthInterceptor {
    fn default() -> Self {
        Self { state: None }
    }
}

impl StatefulAuthInterceptor {
    pub fn with_state(state: AppState) -> Self {
        Self {
            state: Some(state),
        }
    }
}

#[async_trait]
impl SwanInterceptor for StatefulAuthInterceptor {
    // 状态感知的方法实现
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // 首先尝试从context获取state
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                // 从状态中获取缓存的token
                if let Some(token) = app_state.get_cached_token().await {
                    println!("🔐 从AppState获取缓存token: {}...", &token[..std::cmp::min(20, token.len())]);
                    let request_count = app_state.increment_counter().await;
                    println!("📊 这是第 {} 个请求", request_count);
                    
                    let request = request
                        .header("Authorization", format!("Bearer {}", token))
                        .header("X-Request-Count", request_count.to_string());
                    
                    return Ok((request, Cow::Borrowed(request_body)));
                }
            }
        }

        // fallback: 检查内部state
        match &self.state {
            Some(app_state) => {
                // 从状态中获取缓存的token
                if let Some(token) = app_state.get_cached_token().await {
                    println!("🔐 从内部AppState获取缓存token: {}...", &token[..std::cmp::min(20, token.len())]);
                    let request_count = app_state.increment_counter().await;
                    println!("📊 这是第 {} 个请求", request_count);
                    
                    let request = request
                        .header("Authorization", format!("Bearer {}", token))
                        .header("X-Request-Count", request_count.to_string());
                    
                    return Ok((request, Cow::Borrowed(request_body)));
                }
                
                // fallback到默认token
                println!("⚠️  State访问失败，使用fallback");
            }
            None => {
                println!("🔐 使用默认token（无state访问）");
            }
        }
        
        let request = request.header("Authorization", "Bearer default-token");
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        // 首先尝试从context获取state
        if let Some(ctx) = context {
            if let Some(app_state) = ctx.downcast_ref::<AppState>() {
                let current_count = *app_state.request_counter.read().unwrap();
                println!("📈 State统计: 当前已处理 {} 个请求", current_count);
                return Ok(response);
            }
        }

        // fallback: 检查内部state
        if let Some(app_state) = &self.state {
            let current_count = *app_state.request_counter.read().unwrap();
            println!("📈 内部State统计: 当前已处理 {} 个请求", current_count);
        } else {
            println!("✅ 响应处理完成");
        }
        Ok(response)
    }
}


/// 用户API响应
#[derive(Debug, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 带状态的API客户端
/// 
/// 演示state注入：AppState被声明并自动生成相关支持代码
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com", 
    interceptor = StatefulAuthInterceptor,
    state = AppState
)]
struct StatefulApiClient;

impl StatefulApiClient {
    /// 获取用户信息（会使用state中的缓存token）
    #[get(url = "/users/1")]
    async fn get_user(&self) -> anyhow::Result<User> {}

    /// 获取所有用户
    #[get(url = "/users")]
    async fn get_all_users(&self) -> anyhow::Result<Vec<User>> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    println!("=== Swan HTTP State 注入示例 ===\n");
    println!("🎯 演示功能：");
    println!("   1. 应用状态注入 (AppState)");
    println!("   2. 拦截器状态访问 (缓存token、计数器)");
    println!("   3. 链式调用 (.with_state())");
    println!("   4. 状态感知的性能优化\n");

    // 1. 初始化应用状态
    println!("1. 🏗️  初始化应用状态...");
    let app_state = AppState::new();
    let token = app_state.get_cached_token().await.unwrap_or_default();
    println!("   ✅ 缓存token: {}...", 
           if token.len() > 20 { &token[..20] } else { &token });

    // 2. 创建带状态的客户端（链式调用）
    println!("\n2. 🔗 创建带状态的API客户端...");
    let client = StatefulApiClient::new()
        .with_state(app_state.clone());
    println!("   ✅ 客户端已绑定AppState");

    // 3. 测试状态感知的API调用
    println!("\n3. 👤 调用API（拦截器将访问state）...");
    match client.get_user().await {
        Ok(user) => {
            println!("   ✅ 成功获取用户: {}", user.name);
            println!("   📧 邮箱: {}", user.email);
        }
        Err(e) => println!("   ❌ 请求失败: {}", e),
    }

    // 4. 再次调用验证计数器
    println!("\n4. 👥 再次调用API验证计数器...");
    match client.get_all_users().await {
        Ok(users) => {
            println!("   ✅ 成功获取 {} 个用户", users.len());
        }
        Err(e) => println!("   ❌ 请求失败: {}", e),
    }

    // 5. 展示最终状态
    println!("\n5. 📊 最终状态统计:");
    let final_count = *app_state.request_counter.read().unwrap();
    println!("   📈 总请求数: {}", final_count);
    
    println!("\n🎉 State注入示例完成！");
    println!("💡 说明: state在拦截器间共享，支持缓存、数据库访问等复杂场景");
    
    Ok(())
}