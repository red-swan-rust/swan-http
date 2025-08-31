use serde::{Deserialize, Serialize};
use swan_macro::{http_client, get, post, put, delete};
use swan_common::SwanInterceptor;
use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;
use log::{info, warn, error, debug};

/// 用户数据结构
#[derive(Debug, Deserialize, Serialize)]
struct User {
    id: u32,
    name: String,
    email: String,
}

/// 帖子数据结构
#[derive(Debug, Deserialize, Serialize)]
struct Post {
    id: u32,
    #[serde(rename = "userId")]
    user_id: u32,
    title: String,
    body: String,
}

/// 创建帖子请求
#[derive(Serialize)]
struct CreatePostRequest {
    title: String,
    body: String,
    #[serde(rename = "userId")]
    user_id: u32,
}

/// 搜索结果
#[derive(Debug, Deserialize)]
struct SearchResult {
    results: Vec<Post>,
    total: u32,
    page: u32,
}

/// 简单日志拦截器
#[derive(Default)]
struct LoggingInterceptor;

#[async_trait]
impl SwanInterceptor<()> for LoggingInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&()>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        debug!("📝 发送请求...");
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&()>,
    ) -> anyhow::Result<reqwest::Response> {
        info!("✅ 收到响应: {}", response.status());
        Ok(response)
    }
}

/// REST API 客户端 - 演示动态参数功能
#[http_client(
    base_url = "https://jsonplaceholder.typicode.com",
    interceptor = LoggingInterceptor
)]
struct RestApiClient;

impl RestApiClient {
    /// 根据用户ID获取用户信息
    /// URL中的 {user_id} 会被替换为 user_id 参数的值
    #[get(url = "/users/{user_id}")]
    async fn get_user(&self, user_id: u32) -> anyhow::Result<User> {}

    /// 获取用户的所有帖子
    /// URL中的 {user_id} 会被替换为 user_id 参数的值
    #[get(url = "/users/{user_id}/posts")]
    async fn get_user_posts(&self, user_id: u32) -> anyhow::Result<Vec<Post>> {}

    /// 获取特定帖子
    /// URL中的 {post_id} 会被替换为 post_id 参数的值
    #[get(url = "/posts/{post_id}")]
    async fn get_post(&self, post_id: u32) -> anyhow::Result<Post> {}

    /// 为特定用户创建帖子
    /// URL和header中的占位符会被替换
    #[post(
        url = "/users/{user_id}/posts",
        content_type = json,
        header = "X-User-ID: {user_id}",
        header = "X-Content-Source: {source}"
    )]
    async fn create_user_post(
        &self, 
        user_id: u32, 
        source: String, 
        body: CreatePostRequest
    ) -> anyhow::Result<Post> {}

    /// 更新帖子
    /// 演示多个参数在URL中的使用
    #[put(
        url = "/users/{user_id}/posts/{post_id}",
        content_type = json,
        header = "X-User-ID: {user_id}",
        header = "X-Post-ID: {post_id}",
        header = "X-Action: update"
    )]
    async fn update_post(
        &self, 
        user_id: u32, 
        post_id: u32, 
        body: CreatePostRequest
    ) -> anyhow::Result<Post> {}

    /// 删除帖子
    /// header中的 {auth_token} 会被替换为 auth_token 参数的值
    #[delete(
        url = "/posts/{post_id}",
        header = "Authorization: Bearer {auth_token}",
        header = "X-Delete-Reason: {reason}"
    )]
    async fn delete_post(
        &self, 
        post_id: u32, 
        auth_token: String, 
        reason: String
    ) -> anyhow::Result<serde_json::Value> {}

    /// 搜索帖子 - 演示查询参数占位符
    /// 按位置引用参数: {param0} = query, {param1} = page
    #[get(
        url = "/posts?q={param0}&_page={param1}&_limit=10",
        header = "X-Search-Query: {param0}",
        header = "X-Page-Number: {param1}"
    )]
    async fn search_posts_by_position(
        &self, 
        query: String, 
        page: u32
    ) -> anyhow::Result<Vec<Post>> {}

    /// 复杂查询示例 - 混合使用名称和位置占位符
    #[get(
        url = "/posts?userId={user_id}&_page={page}&_sort={sort_field}&_order={order}",
        header = "X-User-ID: {user_id}",
        header = "X-Sort: {sort_field}",
        header = "X-Order: {order}",
        header = "X-Request-ID: {param0}-{param1}-{param2}-{param3}" // 混合引用
    )]
    async fn search_user_posts(
        &self,
        user_id: u32,     // {user_id} 和 {param0}
        page: u32,        // {page} 和 {param1}  
        sort_field: String, // {sort_field} 和 {param2}
        order: String     // {order} 和 {param3}
    ) -> anyhow::Result<Vec<Post>> {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("=== Swan HTTP 动态参数示例 ===\n");
    println!("🎯 演示功能：");
    println!("   1. URL路径参数替换 (/users/{{user_id}})");
    println!("   2. 查询参数替换 (?q={{query}}&page={{page}})");
    println!("   3. Header动态值替换 (Authorization: Bearer {{token}})");
    println!("   4. 按名称引用 ({{user_id}}) 和按位置引用 ({{param0}})");
    println!("   5. 复杂参数组合使用\n");

    let client = RestApiClient::new();

    // 示例1：简单路径参数
    println!("1. 📋 简单路径参数 (/users/{{user_id}})...");
    match client.get_user(1).await {
        Ok(user) => info!("   ✅ 获取用户: {}", user.name),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例2：获取用户帖子
    println!("\n2. 📝 获取用户帖子 (/users/{{user_id}}/posts)...");
    match client.get_user_posts(1).await {
        Ok(posts) => info!("   ✅ 获取到 {} 篇帖子", posts.len()),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例3：创建帖子（URL和header都有动态参数）
    println!("\n3. ✍️  创建帖子（动态URL和header）...");
    let new_post = CreatePostRequest {
        title: "动态参数测试帖子".to_string(),
        body: "这是一个测试帖子，演示动态参数功能".to_string(),
        user_id: 1,
    };
    
    match client.create_user_post(1, "swan-http-client".to_string(), new_post).await {
        Ok(post) => info!("   ✅ 创建帖子: {}", post.title),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例4：更新帖子（多个路径参数）
    println!("\n4. 🔄 更新帖子 (/users/{{user_id}}/posts/{{post_id}})...");
    let update_post = CreatePostRequest {
        title: "更新后的标题".to_string(),
        body: "更新后的内容".to_string(),
        user_id: 1,
    };
    
    match client.update_post(1, 1, update_post).await {
        Ok(post) => info!("   ✅ 更新帖子: {}", post.title),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例5：按位置引用参数
    println!("\n5. 🔍 按位置搜索 (使用 {{param0}}, {{param1}})...");
    match client.search_posts_by_position("swan".to_string(), 1).await {
        Ok(posts) => info!("   ✅ 搜索到 {} 篇帖子", posts.len()),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例6：复杂查询（混合名称和位置引用）
    println!("\n6. 🔍 复杂查询（混合引用方式）...");
    match client.search_user_posts(1, 1, "title".to_string(), "asc".to_string()).await {
        Ok(posts) => info!("   ✅ 查询到 {} 篇帖子", posts.len()),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    // 示例7：删除帖子（动态认证header）
    println!("\n7. 🗑️  删除帖子（动态认证）...");
    match client.delete_post(1, "demo-token-12345".to_string(), "test cleanup".to_string()).await {
        Ok(_) => info!("   ✅ 帖子删除成功"),
        Err(e) => error!("   ❌ 错误: {}", e),
    }

    println!("\n🎉 动态参数示例完成！");
    println!("💡 说明：");
    println!("   - {{param_name}}: 按参数名称引用");
    println!("   - {{param0}}, {{param1}}: 按参数位置引用（从0开始，跳过self）");
    println!("   - 支持URL路径、查询参数、header值的动态替换");
    println!("   - 编译时类型安全，运行时高效替换");
    
    Ok(())
}