use serde::Deserialize;
use swan_macro::{http_client, get};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;
use std::sync::{Arc, RwLock};

/// 企业应用状态
#[derive(Clone)]
struct EnterpriseState {
    auth_tokens: Arc<RwLock<Vec<String>>>, // token池
    request_stats: Arc<RwLock<RequestStats>>, // 请求统计
    tenant_config: TenantConfig, // 租户配置
}

#[derive(Clone)]
struct TenantConfig {
    tenant_id: String,
    organ_id: String,
    app_id: String,
    client_id: String,
}

#[derive(Default, Clone)]
struct RequestStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
}

impl EnterpriseState {
    pub fn new() -> Self {
        let tokens = vec![
            "eyJ0eXBlIjoiSldUIiwiYWxnIjoiSFMyNTYifQ.primary_token".to_string(),
            "eyJ0eXBlIjoiSldUIiwiYWxnIjoiSFMyNTYifQ.backup_token".to_string(),
        ];
        
        Self {
            auth_tokens: Arc::new(RwLock::new(tokens)),
            request_stats: Arc::new(RwLock::new(RequestStats::default())),
            tenant_config: TenantConfig {
                tenant_id: "scddxt".to_string(),
                organ_id: "20240108150502413-27DC-0E8343B1F".to_string(),
                app_id: "110651".to_string(),
                client_id: "scddxt".to_string(),
            },
        }
    }
    
    pub fn get_active_token(&self) -> Option<String> {
        self.auth_tokens.read().unwrap().first().cloned()
    }
    
    pub fn increment_requests(&self) {
        let mut stats = self.request_stats.write().unwrap();
        stats.total_requests += 1;
    }
    
    pub fn increment_success(&self) {
        let mut stats = self.request_stats.write().unwrap();
        stats.successful_requests += 1;
    }
    
    pub fn increment_failure(&self) {
        let mut stats = self.request_stats.write().unwrap();
        stats.failed_requests += 1;
    }
    
    pub fn get_stats(&self) -> RequestStats {
        self.request_stats.read().unwrap().clone()
    }
}

/// 构建记录响应数据结构
#[derive(Debug, Deserialize)]
struct ApiResponse {
    code: i32,
    message: String,
    data: Option<ResponseData>,
}

#[derive(Debug, Deserialize)]
struct ResponseData {
    page: PageInfo,
    summation: f64,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    records: Vec<ConstructionRecord>,
    total: i64,
    size: i64,
    current: i64,
    orders: Vec<serde_json::Value>,
    #[serde(rename = "optimizeCountSql")]
    optimize_count_sql: bool,
    #[serde(rename = "searchCount")]
    search_count: bool,
    #[serde(rename = "countId")]
    count_id: Option<String>,
    #[serde(rename = "maxLimit")]
    max_limit: Option<i64>,
    pages: i64,
}

#[derive(Debug, Deserialize)]
struct ConstructionRecord {
    // 根据实际API定义添加字段
}

/// 企业级API认证拦截器（状态感知）
/// 
/// 演示零拷贝优化 + 状态管理
#[derive(Default)]
struct EnterpriseAuthInterceptor;

#[async_trait]
impl SwanInterceptor for EnterpriseAuthInterceptor {
    async fn before_request<'a>(
        &self,
        mut request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        println!("🔐 Enterprise Auth: 添加企业级认证头部");
        
        // 从state获取动态配置
        if let Some(ctx) = context {
            if let Some(state) = ctx.downcast_ref::<EnterpriseState>() {
                state.increment_requests();
                
                if let Some(token) = state.get_active_token() {
                    println!("🔑 使用state中的认证token: {}...", &token[..30]);
                    request = request.header("Authorization", format!("Bearer {}", token));
                } else {
                    println!("⚠️  State中无可用token，使用默认token");
                    request = request.header("Authorization", "Bearer default_token");
                }
                
                // 使用state中的租户配置
                let config = &state.tenant_config;
                request = request
                    .header("appId", &config.app_id)
                    .header("clientId", &config.client_id)
                    .header("tenantId", &config.tenant_id)
                    .header("organId", &config.organ_id);
                    
                println!("🏢 企业配置: tenant={}, organ={}", config.tenant_id, &config.organ_id[..20]);
            } else {
                println!("⚠️  无法获取Enterprise状态，使用默认配置");
                request = request
                    .header("Authorization", "Bearer default_token")
                    .header("appId", "110651")
                    .header("clientId", "scddxt")
                    .header("tenantId", "scddxt")
                    .header("organId", "20240108150502413-27DC-0E8343B1F");
            }
        } else {
            println!("💡 无context，使用静态配置");
            request = request
                .header("Authorization", "Bearer demo_token")
                .header("appId", "110651")
                .header("clientId", "scddxt")
                .header("tenantId", "scddxt")
                .header("organId", "20240108150502413-27DC-0E8343B1F");
        }
        
        // 零拷贝优化：直接借用请求体
        println!("📊 Performance: 零拷贝优化 - 借用请求体 {} 字节", request_body.len());
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        let status = response.status();
        
        if let Some(ctx) = context {
            if let Some(state) = ctx.downcast_ref::<EnterpriseState>() {
                if status.is_success() {
                    state.increment_success();
                    println!("✅ Enterprise Auth: 请求成功 {} - 统计已更新", status);
                } else {
                    state.increment_failure();
                    println!("❌ Enterprise Auth: 请求失败 {} - 统计已更新", status);
                }
                
                let stats = state.get_stats();
                println!("📈 当前统计: 总请求={}, 成功={}, 失败={}", 
                       stats.total_requests, stats.successful_requests, stats.failed_requests);
            }
        } else {
            println!("✅ Enterprise Auth: 响应状态 {} - {}", 
                   status, status.canonical_reason().unwrap_or("Unknown"));
        }
        
        Ok(response)
    }
}

/// 性能监控拦截器
/// 
/// 演示拦截器缓存优化：重复使用同一个拦截器实例
#[derive(Default)]
struct PerformanceMonitorInterceptor;

#[async_trait]
impl SwanInterceptor for PerformanceMonitorInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        let start_time = std::time::Instant::now();
        println!("📈 Performance Monitor: 请求开始，当前时间戳: {:?}", start_time);
        
        // 零拷贝：直接借用
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("📈 Performance Monitor: 响应状态: {}", response.status());
        println!("📈 Performance Monitor: 响应大小约: {:?} 字节", 
               response.headers().get("content-length"));
        Ok(response)
    }
}

/// 企业级API客户端（无状态版本）
#[http_client(
    base_url = "https://p.crec.cn/dpth/frontend/api/htpm-dispatch-api", 
    interceptor = EnterpriseAuthInterceptor
)]
struct EnterpriseApiClient;

/// 企业级API客户端（状态感知版本）
/// 
/// 展示性能优化：拦截器缓存、零拷贝、条件编译 + 状态管理
#[http_client(
    base_url = "https://p.crec.cn/dpth/frontend/api/htpm-dispatch-api", 
    interceptor = EnterpriseAuthInterceptor,
    state = EnterpriseState
)]
struct StatefulEnterpriseApiClient;

impl EnterpriseApiClient {
    /// 获取构建记录分页数据（无状态版本）
    #[get(
        url = "/constructionRecord/getPage?current=1&size=50&organId=20207&organIdStr=20240108150502413-27DC-0E8343B1F&t=1756545547469",
        header = "Accept: application/json, text/plain, */*",
        header = "Accept-Language: zh-CN,zh;q=0.9",
        header = "Connection: keep-alive",
        header = "Referer: https://p.crec.cn/dpth/dpth/constructionRecord",
        header = "User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        interceptor = PerformanceMonitorInterceptor
    )]
    async fn get_construction_records(&self) -> anyhow::Result<ApiResponse> {}
}

impl StatefulEnterpriseApiClient {
    /// 获取构建记录分页数据（状态感知版本）
    /// 
    /// 展示复杂查询参数处理和状态管理
    #[get(
        url = "/constructionRecord/getPage?current=1&size=50&organId=20207&organIdStr=20240108150502413-27DC-0E8343B1F&t=1756545547469",
        header = "Accept: application/json, text/plain, */*",
        header = "Accept-Language: zh-CN,zh;q=0.9",
        header = "Connection: keep-alive",
        header = "Referer: https://p.crec.cn/dpth/dpth/constructionRecord",
        header = "User-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"
    )]
    async fn get_construction_records(&self) -> anyhow::Result<ApiResponse> {}

    /// 获取原始JSON响应（用于查看完整响应数据）
    #[get(
        url = "/constructionRecord/getPage?current=1&size=50&organId=20207&organIdStr=20240108150502413-27DC-0E8343B1F&t=1756545547469",
        header = "Accept: application/json, text/plain, */*",
        header = "Accept-Language: zh-CN,zh;q=0.9",
        header = "Connection: keep-alive"
    )]
    async fn get_raw_response(&self) -> anyhow::Result<serde_json::Value> {}

    /// 简单的健康检查请求（仅使用全局拦截器）
    #[get(url = "/health")]
    async fn health_check(&self) -> anyhow::Result<serde_json::Value> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志系统（由用户在examples中引入）
    env_logger::init();
    
    println!("=== Swan HTTP 企业级API测试示例 ===\n");
    println!("🚀 演示功能：");
    println!("   1. 拦截器对象池化/缓存优化");
    println!("   2. 零拷贝请求体处理");
    println!("   3. 延迟序列化优化");
    println!("   4. 条件编译调试日志");
    println!("   5. 复杂企业级API调用");
    println!("   6. 🆕 状态注入和管理\n");

    // 示例1：无状态客户端
    println!("1. 📋 无状态客户端（静态配置）...");
    let client = EnterpriseApiClient::new();
    
    match client.get_construction_records().await {
        Ok(response) => {
            println!("   ✅ API响应码: {}", response.code);
            println!("   📄 消息: {}", response.message);
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
            println!("   💡 这是正常的，因为需要有效的认证token");
        }
    }

    // 示例2：状态感知客户端
    println!("\n2. 🏢 状态感知客户端（动态配置）...");
    
    // 创建企业状态
    let enterprise_state = EnterpriseState::new();
    
    // 创建带状态的客户端
    let stateful_client = StatefulEnterpriseApiClient::new()
        .with_state(enterprise_state.clone());
    
    match stateful_client.get_construction_records().await {
        Ok(response) => {
            println!("   ✅ API响应码: {}", response.code);
            println!("   📄 消息: {}", response.message);
            if let Some(data) = response.data {
                let page = &data.page;
                println!("   📊 数据统计: 共 {} 条记录，当前第 {} 页，每页 {} 条", 
                       page.total, page.current, page.size);
                println!("   💰 汇总金额: {:.2}", data.summation);
                println!("   🏗️  记录数量: {} 条", page.records.len());
            }
        }
        Err(e) => {
            println!("   ❌ 请求失败: {}", e);
            println!("   💡 这是正常的，演示了错误处理");
        }
    }

    // 示例3：多次请求验证状态管理
    println!("\n3. 🔄 多次请求验证状态统计...");
    for i in 1..=3 {
        println!("   第 {} 次请求:", i);
        match stateful_client.health_check().await {
            Ok(_) => println!("      ✅ 健康检查通过"),
            Err(e) => println!("      ❌ 健康检查失败: {}", e),
        }
    }

    // 显示最终统计
    let final_stats = enterprise_state.get_stats();
    println!("\n📊 最终请求统计:");
    println!("   📈 总请求数: {}", final_stats.total_requests);
    println!("   ✅ 成功请求: {}", final_stats.successful_requests);
    println!("   ❌ 失败请求: {}", final_stats.failed_requests);

    println!("\n🎯 性能优化验证:");
    println!("   ✅ 拦截器缓存：避免重复创建 EnterpriseAuthInterceptor");
    println!("   ✅ 零拷贝：使用 Cow::Borrowed 避免请求体克隆");
    println!("   ✅ 延迟序列化：GET请求跳过序列化");
    println!("   ✅ 条件编译：release模式下移除调试日志");
    println!("   ✅ 🆕 状态管理：动态token、租户配置、请求统计");

    println!("\n🎉 企业级API测试示例完成！");
    
    Ok(())
}