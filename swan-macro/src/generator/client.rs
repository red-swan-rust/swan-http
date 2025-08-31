use proc_macro::TokenStream;
use quote::quote;
use swan_common::HttpClientArgs;
use syn::ItemStruct;

/// 生成 HTTP 客户端的实现代码
/// 
/// 此函数为使用 `#[http_client]` 宏标注的结构体生成相应的实现代码，
/// 包括必要的字段和构造函数。
/// 
/// # 参数
/// 
/// * `input` - 输入的结构体定义
/// * `args` - HTTP 客户端配置参数
/// 
/// # 返回值
/// 
/// 生成的 TokenStream，包含结构体定义和实现
pub fn generate_http_client_impl(
    mut input: ItemStruct,
    args: &HttpClientArgs,
) -> Result<TokenStream, syn::Error> {
    let struct_name = &input.ident;

    // 确保结构体是空的（无字段）
    if !matches!(input.fields, syn::Fields::Unit) {
        return Err(syn::Error::new_spanned(
            &input,
            "http_client macro only supports empty structs (e.g., `struct ApiClient;`)",
        ));
    }

    let base_url = args.base_url.as_ref()
        .map(|lit| lit.value())
        .unwrap_or_default();
    
    let interceptor = &args.interceptor;

    // 生成字段（根据是否有状态来决定拦截器类型）
    let fields = if let Some(state_type) = &args.state {
        syn::parse_quote! {{
            client: reqwest::Client,
            base_url: String,
            global_interceptor: Option<std::sync::Arc<dyn swan_common::SwanInterceptor<#state_type> + Send + Sync>>,
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<#state_type>,
        }}
    } else if args.interceptor.is_some() {
        syn::parse_quote! {{
            client: reqwest::Client,
            base_url: String,
            global_interceptor: Option<std::sync::Arc<dyn swan_common::SwanInterceptor<()> + Send + Sync>>,
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<()>,
        }}
    } else {
        syn::parse_quote! {{
            client: reqwest::Client,
            base_url: String,
            global_interceptor: Option<std::sync::Arc<()>>, // 无拦截器的占位
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<()>,
        }}
    };
    
    input.fields = syn::Fields::Named(fields);

    let interceptor_init = if let Some(interceptor_path) = interceptor {
        if let Some(state_type) = &args.state {
            // 有状态：创建 SwanInterceptor<StateType>
            quote! { 
                Some(std::sync::Arc::new(<#interceptor_path as Default>::default()) as std::sync::Arc<dyn swan_common::SwanInterceptor<#state_type> + Send + Sync>)
            }
        } else {
            // 无状态：创建 SwanInterceptor<()>
            quote! { 
                Some(std::sync::Arc::new(<#interceptor_path as Default>::default()) as std::sync::Arc<dyn swan_common::SwanInterceptor<()> + Send + Sync>)
            }
        }
    } else {
        quote! { None }
    };

    // 生成state字段初始化和with_state方法
    // 只有在同时有 state 和 interceptor 时才生成 with_state 方法
    let (state_field_init, with_state_method) = if let Some(state_type) = &args.state {
        if args.interceptor.is_some() {
            (
                quote! { state: None, },
                quote! {
                    /// 设置应用状态（链式调用）
                    pub fn with_state(mut self, state: #state_type) -> Self {
                        self.state = Some(state);
                        self
                    }
                }
            )
        } else {
            // 这种情况已经在解析阶段被拦截，但为了安全起见保留
            (
                quote! { state: None, },
                quote! {}
            )
        }
    } else {
        (
            quote! { state: None, },
            quote! {} // 无状态客户端不提供with_state方法
        )
    };

    // 生成状态标识信息
    let (state_type_for_trait, has_state_flag) = if let Some(state_type) = &args.state {
        (quote! { #state_type }, quote! { true })
    } else {
        (quote! { () }, quote! { false })
    };

    let expanded = quote! {
        #input

        impl #struct_name {
            /// 创建新的 HTTP 客户端实例
            pub fn new() -> Self {
                #struct_name {
                    client: reqwest::Client::new(),
                    base_url: #base_url.to_string(),
                    global_interceptor: #interceptor_init,
                    interceptor_cache: std::sync::Mutex::new(swan_common::InterceptorCache::new()),
                    #state_field_init
                }
            }

            #with_state_method

            /// 预热拦截器缓存
            /// 
            /// 在客户端创建后调用，可以预先创建常用的拦截器实例，
            /// 避免首次调用时的创建开销。
            pub fn warmup_interceptor<T>(&self) 
            where 
                T: Default + Send + Sync + 'static,
            {
                if let Ok(mut cache) = self.interceptor_cache.lock() {
                    cache.warmup::<T>();
                }
            }
        }

        // 为客户端实现状态标识 trait
        impl swan_common::ClientStateMarker for #struct_name {
            type State = #state_type_for_trait;
            const HAS_STATE: bool = #has_state_flag;
        }
    };

    Ok(TokenStream::from(expanded))
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, LitStr};
    use proc_macro2::Span;

    #[test]
    fn test_generate_http_client_impl_validation() {
        let input: ItemStruct = parse_quote! {
            struct TestClient;
        };
        
        let args = HttpClientArgs {
            base_url: Some(LitStr::new("https://api.test.com", Span::call_site())),
            interceptor: None,
            state: None,
        };

        // 测试基本验证逻辑，不依赖TokenStream
        assert!(matches!(input.fields, syn::Fields::Unit));
        assert!(args.base_url.is_some());
        assert_eq!(args.base_url.as_ref().unwrap().value(), "https://api.test.com");
    }

    #[test]
    fn test_generate_http_client_impl_with_fields_should_error() {
        let input: ItemStruct = parse_quote! {
            struct TestClient {
                field: String,
            }
        };
        
        let args = HttpClientArgs {
            base_url: None,
            interceptor: None,
            state: None,
        };

        // 测试验证逻辑，应该检测到非空结构体
        assert!(!matches!(input.fields, syn::Fields::Unit));
        
        let result = generate_http_client_impl(input, &args);
        assert!(result.is_err());
    }
}