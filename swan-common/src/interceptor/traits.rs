use async_trait::async_trait;
use std::borrow::Cow;
use std::any::Any;

/// 新的统一拦截器接口（基于零拷贝设计）
///
/// 这是 Swan HTTP 库的高性能拦截器接口，使用 Cow 来避免不必要的内存拷贝。
///
/// # 示例
///
/// ```rust
/// use async_trait::async_trait;
/// use swan_common::SwanInterceptor;
/// use std::borrow::Cow;
/// use std::any::Any;
///
/// struct AuthInterceptor {
///     token: String,
/// }
///
/// #[async_trait]
/// impl SwanInterceptor for AuthInterceptor {
///     async fn before_request<'a>(
///         &self,
///         mut request: reqwest::RequestBuilder,
///         request_body: &'a [u8],
///         _context: Option<&(dyn Any + Send + Sync)>,
///     ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
///         request = request.header("Authorization", format!("Bearer {}", self.token));
///         Ok((request, Cow::Borrowed(request_body)))
///     }
///
///     async fn after_response(
///         &self,
///         response: reqwest::Response,
///         _context: Option<&(dyn Any + Send + Sync)>,
///     ) -> anyhow::Result<reqwest::Response> {
///         Ok(response)
///     }
/// }
/// ```
#[async_trait]
pub trait SwanInterceptor {
    /// 零拷贝的请求前处理
    /// 
    /// 使用 Cow 避免不必要的数据拷贝，只有在真正需要修改时才进行克隆
    /// 
    /// # 参数
    /// 
    /// * `request` - 请求构建器
    /// * `request_body` - 请求体的借用引用
    /// * `context` - 可选的应用状态上下文
    /// 
    /// # 返回值
    /// 
    /// 返回修改后的请求构建器和可能修改的请求体
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>;

    /// 响应后处理
    /// 
    /// # 参数
    /// 
    /// * `response` - HTTP 响应
    /// * `context` - 可选的应用状态上下文
    /// 
    /// # 返回值
    /// 
    /// 返回可能修改的响应
    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response>;

}


/// 空拦截器实现，用于测试和默认情况
#[derive(Default)]
pub struct NoOpInterceptor;

#[async_trait]
impl SwanInterceptor for NoOpInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_no_op_interceptor() {
        let interceptor = NoOpInterceptor;
        let client = Client::new();
        let request = client.get("https://httpbin.org/get");
        let body = vec![1, 2, 3];

        let (_modified_request, modified_body) = interceptor
            .before_request(request, &body, None)
            .await
            .unwrap();
        
        assert_eq!(modified_body, body);
        
        // 注意：在实际测试中，我们需要模拟响应而不是发送真实请求
    }
}