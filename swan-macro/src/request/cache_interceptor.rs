use quote::quote;
use syn::Path;

/// 缓存式拦截器处理器
/// 
/// 使用客户端级缓存来管理拦截器实例，避免重复创建
pub struct CachedInterceptorProcessor;

impl CachedInterceptorProcessor {
    /// 生成缓存式拦截器获取代码
    /// 
    /// # 参数
    /// 
    /// * `interceptor_path` - 拦截器类型路径
    /// * `state_type` - 状态类型（可选）
    /// 
    /// # 返回值
    /// 
    /// 生成的拦截器获取代码
    pub fn generate_cached_interceptor_access(
        interceptor_path: &Option<Path>, 
        _state_type: Option<&syn::Type>
    ) -> proc_macro2::TokenStream {
        match interceptor_path {
            Some(path) => {
                quote! {
                    let method_interceptor = {
                        let mut cache = self.interceptor_cache.lock().unwrap();
                        Some(cache.get_or_create::<#path>())
                    };
                }
            },
            None => {
                quote! {
                    let method_interceptor: Option<std::sync::Arc<()>> = None;
                }
            },
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_cached_interceptor_access_with_path() {
        let path: Path = parse_quote! { MyInterceptor };
        let result = CachedInterceptorProcessor::generate_cached_interceptor_access(&Some(path), None);
        
        let result_str = result.to_string();
        assert!(result_str.contains("MyInterceptor"));
        assert!(result_str.contains("get_or_create"));
        assert!(result_str.contains("interceptor_cache"));
    }

    #[test]
    fn test_generate_cached_interceptor_access_none() {
        let result = CachedInterceptorProcessor::generate_cached_interceptor_access(&None, None);
        let result_str = result.to_string();
        assert!(result_str.contains("None"));
    }

}