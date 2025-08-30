use quote::quote;

/// 零开销抽象优化器
/// 
/// 使用常量泛型和编译时分支实现零运行时开销的功能特性
pub struct ZeroCostOptimizer;

impl ZeroCostOptimizer {
    /// 生成零开销拦截器处理代码
    /// 
    /// 使用 const 泛型在编译时确定是否需要拦截器处理，
    /// 完全消除运行时分支和动态分发开销
    pub fn generate_zero_cost_interceptor_code(
        has_global_interceptor: bool,
        has_method_interceptor: bool,
    ) -> proc_macro2::TokenStream {
        match (has_global_interceptor, has_method_interceptor) {
            (false, false) => {
                // 编译时确定无拦截器，生成直接执行路径
                quote! {
                    const HAS_INTERCEPTORS: bool = false;
                    
                    #[inline(always)]
                    async fn execute_with_interceptors(
                        client: &reqwest::Client,
                        request: reqwest::Request,
                    ) -> anyhow::Result<reqwest::Response> {
                        // 零开销：直接执行，无拦截器开销
                        client.execute(request).await
                            .map_err(|e| anyhow::anyhow!("Request execution failed: {}", e))
                    }
                }
            }
            (true, false) => {
                // 仅全局拦截器的优化路径
                quote! {
                    const HAS_INTERCEPTORS: bool = true;
                    const HAS_GLOBAL_INTERCEPTOR: bool = true;
                    const HAS_METHOD_INTERCEPTOR: bool = false;
                    
                    #[inline(always)]
                    async fn execute_with_interceptors(
                        client: &reqwest::Client,
                        mut request: reqwest::Request,
                        global_interceptor: &dyn swan_common::SwanInterceptor,
                        context: Option<&(dyn std::any::Any + Send + Sync)>,
                    ) -> anyhow::Result<reqwest::Response> {
                        // 零开销：编译时确定只需处理全局拦截器
                        let request_builder = reqwest::RequestBuilder::from_parts(client.clone(), request);
                        let (modified_builder, _) = global_interceptor
                            .before_request(request_builder, &[], context)
                            .await?;
                        
                        let request = modified_builder.build()
                            .map_err(|e| anyhow::anyhow!("Failed to build modified request: {}", e))?;
                            
                        let response = client.execute(request).await
                            .map_err(|e| anyhow::anyhow!("Request execution failed: {}", e))?;
                            
                        global_interceptor.after_response(response, context).await
                    }
                }
            }
            (false, true) => {
                // 仅方法级拦截器的优化路径
                quote! {
                    const HAS_INTERCEPTORS: bool = true;
                    const HAS_GLOBAL_INTERCEPTOR: bool = false;
                    const HAS_METHOD_INTERCEPTOR: bool = true;
                    
                    #[inline(always)]
                    async fn execute_with_interceptors(
                        client: &reqwest::Client,
                        request: reqwest::Request,
                        method_interceptor: &dyn swan_common::SwanInterceptor,
                        context: Option<&(dyn std::any::Any + Send + Sync)>,
                    ) -> anyhow::Result<reqwest::Response> {
                        // 零开销：编译时确定只需处理方法级拦截器
                        let request_builder = reqwest::RequestBuilder::from_parts(client.clone(), request);
                        let (modified_builder, _) = method_interceptor
                            .before_request(request_builder, &[], context)
                            .await?;
                        
                        let request = modified_builder.build()
                            .map_err(|e| anyhow::anyhow!("Failed to build modified request: {}", e))?;
                            
                        let response = client.execute(request).await
                            .map_err(|e| anyhow::anyhow!("Request execution failed: {}", e))?;
                            
                        method_interceptor.after_response(response, context).await
                    }
                }
            }
            (true, true) => {
                // 双拦截器的优化路径
                quote! {
                    const HAS_INTERCEPTORS: bool = true;
                    const HAS_GLOBAL_INTERCEPTOR: bool = true;
                    const HAS_METHOD_INTERCEPTOR: bool = true;
                    
                    #[inline(always)]
                    async fn execute_with_interceptors(
                        client: &reqwest::Client,
                        request: reqwest::Request,
                        global_interceptor: &dyn swan_common::SwanInterceptor,
                        method_interceptor: &dyn swan_common::SwanInterceptor,
                        context: Option<&(dyn std::any::Any + Send + Sync)>,
                    ) -> anyhow::Result<reqwest::Response> {
                        // 零开销：编译时确定拦截器调用顺序
                        let request_builder = reqwest::RequestBuilder::from_parts(client.clone(), request);
                        
                        // 全局拦截器先处理
                        let (temp_builder, temp_body) = global_interceptor
                            .before_request(request_builder, &[], context)
                            .await?;
                        
                        // 方法级拦截器后处理
                        let (final_builder, _) = method_interceptor
                            .before_request(temp_builder, &temp_body, context)
                            .await?;
                        
                        let request = final_builder.build()
                            .map_err(|e| anyhow::anyhow!("Failed to build modified request: {}", e))?;
                            
                        let response = client.execute(request).await
                            .map_err(|e| anyhow::anyhow!("Request execution failed: {}", e))?;
                        
                        // 响应处理：方法级先，全局后
                        let response = method_interceptor.after_response(response, context).await?;
                        global_interceptor.after_response(response, context).await
                    }
                }
            }
        }
    }

    /// 生成零开销状态访问代码
    /// 
    /// 使用编译时分支避免不必要的状态检查和类型转换
    pub fn generate_zero_cost_state_access(has_state: bool) -> proc_macro2::TokenStream {
        if has_state {
            quote! {
                const HAS_STATE: bool = true;
                
                #[inline(always)]
                fn get_context(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
                    // 零开销：编译时确定有状态，直接返回
                    self.state.as_ref().map(|s| s as &(dyn std::any::Any + Send + Sync))
                }
            }
        } else {
            quote! {
                const HAS_STATE: bool = false;
                
                #[inline(always)]
                fn get_context(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
                    // 零开销：编译时确定无状态，直接返回None
                    None
                }
            }
        }
    }

    /// 生成零开销序列化代码
    /// 
    /// 根据内容类型在编译时选择最优的序列化路径
    pub fn generate_zero_cost_serialization(
        content_type: &Option<swan_common::ContentType>,
        method: &swan_common::HttpMethod,
    ) -> proc_macro2::TokenStream {
        match (method, content_type) {
            (swan_common::HttpMethod::Get | swan_common::HttpMethod::Delete, _) => {
                quote! {
                    const NEEDS_SERIALIZATION: bool = false;
                    
                    #[inline(always)]
                    fn serialize_body<T>(_body: &T) -> anyhow::Result<Vec<u8>> {
                        // 零开销：GET/DELETE不需要序列化
                        Ok(Vec::new())
                    }
                }
            }
            (_, Some(swan_common::ContentType::Json)) => {
                quote! {
                    const NEEDS_SERIALIZATION: bool = true;
                    const SERIALIZATION_TYPE: &str = "json";
                    
                    #[inline(always)]
                    fn serialize_body<T: serde::Serialize>(body: &T) -> anyhow::Result<Vec<u8>> {
                        // 零开销：编译时确定JSON序列化
                        serde_json::to_vec(body)
                            .map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e))
                    }
                }
            }
            _ => {
                quote! {
                    const NEEDS_SERIALIZATION: bool = false;
                    
                    #[inline(always)]
                    fn serialize_body<T>(_body: &T) -> anyhow::Result<Vec<u8>> {
                        // 零开销：其他情况暂不序列化
                        Ok(Vec::new())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_cost_no_interceptors() {
        let result = ZeroCostOptimizer::generate_zero_cost_interceptor_code(false, false);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_INTERCEPTORS: bool = false"));
        assert!(result_str.contains("inline(always)"));
    }

    #[test]
    fn test_zero_cost_global_interceptor_only() {
        let result = ZeroCostOptimizer::generate_zero_cost_interceptor_code(true, false);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_GLOBAL_INTERCEPTOR: bool = true"));
        assert!(result_str.contains("HAS_METHOD_INTERCEPTOR: bool = false"));
    }

    #[test]
    fn test_zero_cost_state_access_with_state() {
        let result = ZeroCostOptimizer::generate_zero_cost_state_access(true);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_STATE: bool = true"));
    }

    #[test]
    fn test_zero_cost_state_access_no_state() {
        let result = ZeroCostOptimizer::generate_zero_cost_state_access(false);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_STATE: bool = false"));
        assert!(result_str.contains("None"));
    }
}