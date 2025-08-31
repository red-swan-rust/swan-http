use async_trait::async_trait;
use std::borrow::Cow;

/// 客户端状态类型标识 trait
pub trait ClientStateMarker {
    type State;
    const HAS_STATE: bool;
}

/// Swan HTTP 无状态拦截器接口
///
/// 用于不需要状态管理的拦截器，接口更简洁，无需处理state参数
#[async_trait]
pub trait SwanInterceptor {
    /// 请求前处理
    /// 
    /// # 参数
    /// - `request`: 请求构建器
    /// - `request_body`: 请求体字节数组（零拷贝）
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>;

    /// 响应后处理
    /// 
    /// # 参数  
    /// - `response`: HTTP响应对象
    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response>;
}

/// Swan HTTP 有状态拦截器接口
///
/// 用于需要状态管理的拦截器，提供类型安全的状态访问
#[async_trait]
pub trait SwanStatefulInterceptor<State> {
    /// 请求前处理
    /// 
    /// # 参数
    /// - `request`: 请求构建器
    /// - `request_body`: 请求体字节数组（零拷贝）
    /// - `state`: 状态对象（类型安全）
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        state: Option<&State>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)>;

    /// 响应后处理
    /// 
    /// # 参数  
    /// - `response`: HTTP响应对象
    /// - `state`: 状态对象（类型安全）
    async fn after_response(
        &self,
        response: reqwest::Response,
        state: Option<&State>,
    ) -> anyhow::Result<reqwest::Response>;
}

/// 空拦截器实现，用于测试和默认情况
#[derive(Default)]
pub struct NoOpInterceptor;

// 无状态空拦截器实现
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

// 有状态空拦截器实现（支持任意状态类型）
#[async_trait]
impl<State> SwanStatefulInterceptor<State> for NoOpInterceptor 
where 
    State: Send + Sync,
{
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        _state: Option<&State>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        _state: Option<&State>,
    ) -> anyhow::Result<reqwest::Response> {
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Client;

    #[tokio::test]
    async fn test_stateless_no_op_interceptor() {
        let interceptor = NoOpInterceptor;
        let client = Client::new();
        let request = client.get("https://httpbin.org/get");
        let body = vec![1, 2, 3];

        // 无状态：使用简洁的接口，无state参数
        let (_modified_request, modified_body) = SwanInterceptor::before_request(&interceptor, request, &body)
            .await
            .unwrap();
        
        assert_eq!(modified_body, body);
    }

    #[tokio::test]
    async fn test_stateful_no_op_interceptor() {
        struct TestState {
            value: i32,
        }
        
        let interceptor = NoOpInterceptor;
        let client = Client::new();
        let request = client.get("https://httpbin.org/get");
        let body = vec![1, 2, 3];
        let state = TestState { value: 42 };

        // 有状态：使用具体的状态类型
        let (_modified_request, modified_body) = SwanStatefulInterceptor::before_request(&interceptor, request, &body, Some(&state))
            .await
            .unwrap();
        
        assert_eq!(modified_body, body);
    }
}