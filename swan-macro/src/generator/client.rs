use proc_macro::TokenStream;
use quote::quote;
use swan_common::{HttpClientArgs, ProxyConfig, ProxyType};
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
            global_interceptor: Option<std::sync::Arc<dyn swan_common::SwanStatefulInterceptor<#state_type> + Send + Sync>>,
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<#state_type>,
        }}
    } else if args.interceptor.is_some() {
        syn::parse_quote! {{
            client: reqwest::Client,
            base_url: String,
            global_interceptor: Option<std::sync::Arc<dyn swan_common::SwanInterceptor + Send + Sync>>,
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<()>,
        }}
    } else {
        syn::parse_quote! {{
            client: reqwest::Client,
            base_url: String,
            interceptor_cache: std::sync::Mutex<swan_common::InterceptorCache>,
            state: Option<()>,
        }}
    };
    
    input.fields = syn::Fields::Named(fields);

    let interceptor_init = if let Some(interceptor_path) = interceptor {
        if let Some(state_type) = &args.state {
            // 有状态：创建 SwanStatefulInterceptor<StateType>
            quote! { 
                global_interceptor: Some(std::sync::Arc::new(<#interceptor_path as Default>::default()) as std::sync::Arc<dyn swan_common::SwanStatefulInterceptor<#state_type> + Send + Sync>),
            }
        } else {
            // 无状态：创建 SwanInterceptor
            quote! { 
                global_interceptor: Some(std::sync::Arc::new(<#interceptor_path as Default>::default()) as std::sync::Arc<dyn swan_common::SwanInterceptor + Send + Sync>),
            }
        }
    } else {
        quote! {}  // 无拦截器时不生成global_interceptor字段初始化
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

    // 条件性trait导出 - 关键功能！
    let conditional_trait_export = if args.state.is_some() && args.interceptor.is_some() {
        // 有状态时只导出SwanStatefulInterceptor
        quote! {
            pub use swan_common::SwanStatefulInterceptor;
        }
    } else if args.interceptor.is_some() {
        // 无状态时只导出SwanInterceptor
        quote! {
            pub use swan_common::SwanInterceptor;
        }
    } else {
        // 无拦截器时不导出
        quote! {}
    };

    // 生成客户端创建代码（根据代理配置）
    let client_creation = generate_client_creation(&args.proxy)?;

    let expanded = quote! {
        #conditional_trait_export

        #input

        impl #struct_name {
            /// 创建新的 HTTP 客户端实例
            pub fn new() -> Self {
                #struct_name {
                    client: #client_creation,
                    base_url: #base_url.to_string(),
                    #interceptor_init
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

/// 生成客户端创建代码（根据代理配置）
fn generate_client_creation(proxy_config: &Option<ProxyConfig>) -> Result<proc_macro2::TokenStream, syn::Error> {
    match proxy_config {
        None => {
            // 无代理配置，使用默认客户端
            Ok(quote! { reqwest::Client::new() })
        }
        Some(ProxyConfig::Disabled(_)) => {
            // 明确禁用代理
            Ok(quote! {
                reqwest::Client::builder()
                    .no_proxy()
                    .build()
                    .unwrap_or_else(|e| panic!("Failed to create HTTP client with no proxy: {}", e))
            })
        }
        Some(proxy_config @ ProxyConfig::Simple(_)) => {
            let url = proxy_config.url().unwrap();
            let url_value = &url.value();
            
            match proxy_config.infer_proxy_type() {
                Some(ProxyType::Http) => {
                    Ok(quote! {
                        {
                            let proxy_url = #url_value;
                            let proxy = reqwest::Proxy::all(proxy_url)
                                .unwrap_or_else(|e| panic!("Invalid HTTP proxy URL '{}': {}", proxy_url, e));
                            
                            reqwest::Client::builder()
                                .proxy(proxy)
                                .build()
                                .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                        }
                    })
                }
                Some(ProxyType::Socks5) => {
                    Ok(quote! {
                        {
                            let proxy_url = #url_value;
                            let proxy = reqwest::Proxy::all(proxy_url)
                                .unwrap_or_else(|e| panic!("Invalid SOCKS5 proxy URL '{}': {}", proxy_url, e));
                            
                            reqwest::Client::builder()
                                .proxy(proxy)
                                .build()
                                .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                        }
                    })
                }
                None => {
                    Err(syn::Error::new_spanned(
                        url,
                        "Cannot infer proxy type from URL. Use proxy(type = http/socks5, url = \"...\") format or ensure URL starts with http://, https://, or socks5://"
                    ))
                }
            }
        }
        Some(proxy_config @ ProxyConfig::Full { url, username, password, no_proxy, .. }) => {
            let url_value = &url.value();
            let username_value = username.as_ref().map(|u| u.value());
            let password_value = password.as_ref().map(|p| p.value());
            let no_proxy_value = no_proxy.as_ref().map(|np| np.value());

            match proxy_config.infer_proxy_type() {
                Some(ProxyType::Http) => {
                    // HTTP 代理配置
                    let auth_code = match (username_value, password_value) {
                        (Some(username), Some(password)) => {
                            quote! {
                                proxy = proxy.basic_auth(#username, #password);
                            }
                        }
                        _ => quote! {}
                    };
                    
                    let no_proxy_code = match no_proxy_value {
                        Some(no_proxy_domains) => {
                            quote! {
                                eprintln!("Warning: no_proxy configuration '{}' is not directly supported by reqwest. Consider using environment variables.", #no_proxy_domains);
                            }
                        }
                        None => quote! {}
                    };

                    Ok(quote! {
                        {
                            let proxy_url = #url_value;
                            let mut proxy = reqwest::Proxy::all(proxy_url)
                                .unwrap_or_else(|e| panic!("Invalid HTTP proxy URL '{}': {}", proxy_url, e));

                            #auth_code

                            let client_builder = reqwest::Client::builder().proxy(proxy);

                            #no_proxy_code

                            client_builder
                                .build()
                                .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                        }
                    })
                }
                Some(ProxyType::Socks5) => {
                    // SOCKS5 代理配置
                    let auth_code = match (username_value, password_value) {
                        (Some(username), Some(password)) => {
                            quote! {
                                proxy = proxy.basic_auth(#username, #password);
                            }
                        }
                        _ => quote! {}
                    };
                    
                    let no_proxy_code = match no_proxy_value {
                        Some(no_proxy_domains) => {
                            quote! {
                                eprintln!("Warning: no_proxy configuration '{}' is not directly supported by reqwest. Consider using environment variables.", #no_proxy_domains);
                            }
                        }
                        None => quote! {}
                    };

                    Ok(quote! {
                        {
                            let proxy_url = #url_value;
                            let mut proxy = reqwest::Proxy::all(proxy_url)
                                .unwrap_or_else(|e| panic!("Invalid SOCKS5 proxy URL '{}': {}", proxy_url, e));

                            #auth_code

                            let client_builder = reqwest::Client::builder().proxy(proxy);

                            #no_proxy_code

                            client_builder
                                .build()
                                .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                        }
                    })
                }
                None => {
                    Err(syn::Error::new_spanned(
                        url,
                        "Cannot infer proxy type. Use proxy(type = http/socks5, url = \"...\") format or ensure URL starts with http://, https://, or socks5://"
                    ))
                }
            }
        }
    }
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
            proxy: None,
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
            proxy: None,
        };

        // 测试验证逻辑，应该检测到非空结构体
        assert!(!matches!(input.fields, syn::Fields::Unit));
        
        let result = generate_http_client_impl(input, &args);
        assert!(result.is_err());
    }
}