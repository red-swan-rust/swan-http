use proc_macro::TokenStream;
use quote::quote;
use swan_common::{HandlerArgs, ProxyConfig, ProxyType};
use syn::{FnArg, Signature};
use crate::conversion::generate_type_conversion;
use crate::error::ErrorHandler;
use crate::request::{RequestBuilder, CachedInterceptorProcessor, RetryProcessor};
use crate::optimization::ConditionalOptimizer;

/// 生成 HTTP 方法的实现代码
/// 
/// 此函数为使用 HTTP 方法宏（如 `#[get]`, `#[post]` 等）标注的函数生成相应的实现代码。
/// 
/// # 参数
/// 
/// * `fn_sig` - 函数签名
/// * `handler_args` - HTTP 处理器参数
/// 
/// # 返回值
/// 
/// 生成的 TokenStream，包含完整的异步方法实现
/// 
/// # 注意
/// 
/// 目前暂时使用统一的接口，未来需要改进以支持编译时 trait 检测
pub fn generate_http_method(fn_sig: &Signature, handler_args: &HandlerArgs) -> TokenStream {
    // 检查是否在有状态的客户端上下文中 - 通过查看 self 的预期字段类型
    generate_http_method_impl(fn_sig, handler_args, None)
}

pub fn generate_http_method_impl(fn_sig: &Signature, handler_args: &HandlerArgs, client_state_type: Option<&syn::Type>) -> TokenStream {
    let fn_name = &fn_sig.ident;
    let inputs = &fn_sig.inputs;
    let output = &fn_sig.output;

    // 验证函数参数
    if let Err(error) = validate_function_inputs(inputs) {
        return error.to_compile_error().into();
    }

    // 验证并提取返回类型
    let (ok_type, _err_type) = match ErrorHandler::validate_and_extract_return_types(output) {
        Ok(types) => types,
        Err(error) => return error.to_compile_error().into(),
    };

    // 生成函数参数和请求体处理代码
    let (_body_type, body_param, body_method_call) = generate_body_handling(inputs, handler_args);

    // 生成缓存式拦截器处理代码 - 传递状态类型信息
    let method_interceptor_access = CachedInterceptorProcessor::generate_cached_interceptor_access(&handler_args.interceptor, client_state_type);
    
    // 生成客户端选择代码（根据方法级代理配置）
    let client_selection = match generate_client_selection(&handler_args.proxy) {
        Ok(code) => code,
        Err(error) => return error.to_compile_error().into(),
    };
    
    let request_builder_code = RequestBuilder::generate_request_builder_code(handler_args, &body_method_call, inputs);

    // 生成类型转换代码
    let type_conversion = generate_type_conversion(ok_type);

    // 生成延迟序列化代码

    // 生成条件编译优化代码
    let conditional_logging = ConditionalOptimizer::generate_conditional_logging();
    let conditional_response_logging = ConditionalOptimizer::generate_conditional_response_logging();
    
    // 生成重试执行代码
    let retry_execution = RetryProcessor::generate_complete_retry_block(&handler_args.retry, &handler_args.method);

    // 生成拦截器调用代码
    let interceptor_calls = generate_interceptor_calls(&handler_args, client_state_type);

    let expanded = quote! {
        pub async fn #fn_name(&self #body_param) #output {

            #client_selection

            #request_builder_code

            #conditional_logging

            let request = match request_builder.build() {
                Ok(req) => req,
                Err(e) => return Err(anyhow::anyhow!("Failed to build request: {}", e)),
            };

            // 执行请求（包含重试逻辑和响应处理）
            let result = {
                #retry_execution
                
                if response.status().is_success() {
                    let bytes = match response.bytes().await {
                        Ok(bytes) => bytes,
                        Err(e) => return Err(anyhow::anyhow!("Failed to read response bytes: {}", e)),
                    };

                    let result = #type_conversion;
                    #conditional_response_logging
                    Ok(result)
                } else {
                    Err(anyhow::anyhow!("Request failed with status: {}", response.status()))
                }
            };
            
            result
        }
    };

    TokenStream::from(expanded)
}

/// 生成客户端选择代码（支持方法级代理覆盖）
fn generate_client_selection(proxy_config: &Option<ProxyConfig>) -> Result<proc_macro2::TokenStream, syn::Error> {
    match proxy_config {
        None => {
            // 无方法级代理配置，使用实例客户端
            Ok(quote! {
                let effective_client = &self.client;
            })
        }
        Some(ProxyConfig::Disabled(_)) => {
            // 方法级禁用代理，创建临时无代理客户端
            Ok(quote! {
                let effective_client = &{
                    static METHOD_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
                    METHOD_CLIENT.get_or_init(|| {
                        reqwest::Client::builder()
                            .no_proxy()
                            .build()
                            .unwrap_or_else(|e| panic!("Failed to create HTTP client with no proxy: {}", e))
                    })
                };
            })
        }
        Some(proxy_config @ ProxyConfig::Simple(_)) => {
            let url = proxy_config.url().unwrap();
            let url_value = &url.value();
            
            match proxy_config.infer_proxy_type() {
                Some(ProxyType::Http) | Some(ProxyType::Socks5) => {
                    Ok(quote! {
                        let effective_client = &{
                            static METHOD_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
                            METHOD_CLIENT.get_or_init(|| {
                                let proxy_url = #url_value;
                                let proxy = reqwest::Proxy::all(proxy_url)
                                    .unwrap_or_else(|e| panic!("Invalid proxy URL '{}': {}", proxy_url, e));
                                
                                reqwest::Client::builder()
                                    .proxy(proxy)
                                    .build()
                                    .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                            })
                        };
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
                Some(ProxyType::Http) | Some(ProxyType::Socks5) => {
                    // 方法级完整代理配置，创建临时代理客户端
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
                        let effective_client = &{
                            static METHOD_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
                            METHOD_CLIENT.get_or_init(|| {
                                let proxy_url = #url_value;
                                let mut proxy = reqwest::Proxy::all(proxy_url)
                                    .unwrap_or_else(|e| panic!("Invalid proxy URL '{}': {}", proxy_url, e));

                                #auth_code

                                let client_builder = reqwest::Client::builder().proxy(proxy);

                                #no_proxy_code

                                client_builder
                                    .build()
                                    .unwrap_or_else(|e| panic!("Failed to create HTTP client with proxy '{}': {}", proxy_url, e))
                            })
                        };
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

/// 生成拦截器调用代码
/// 暂时简化，移除拦截器调用逻辑
fn generate_interceptor_calls(_handler_args: &HandlerArgs, _client_state_type: Option<&syn::Type>) -> proc_macro2::TokenStream {
    // 暂时移除拦截器调用，专注于trait导出功能
    quote! {
        // 拦截器调用逻辑待后续完善
    }
}

/// 验证函数输入参数
fn validate_function_inputs(inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>) -> Result<(), syn::Error> {
    if inputs.is_empty() {
        return Err(syn::Error::new_spanned(inputs, "method must have at least 'self' parameter"));
    }

    let self_arg = inputs.iter().next().unwrap();
    if !matches!(self_arg, FnArg::Receiver(_)) {
        return Err(syn::Error::new_spanned(
            self_arg,
            "first parameter must be 'self', '&self', or '&mut self'",
        ));
    }

    Ok(())
}


/// 生成请求体处理代码
fn generate_body_handling(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    handler_args: &HandlerArgs,
) -> (proc_macro2::TokenStream, proc_macro2::TokenStream, proc_macro2::TokenStream) {
    // 收集所有非self参数
    let mut param_tokens = Vec::new();
    let mut body_type = None;
    let mut body_method_call = quote! {};

    for (index, input) in inputs.iter().skip(1).enumerate() {
        if let FnArg::Typed(pat_type) = input {
            if let syn::Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let param_name = &pat_ident.ident;
                let param_type = &pat_type.ty;
                
                param_tokens.push(quote! { , #param_name: #param_type });
                
                // 检查是否是body参数（通常是最后一个参数，且方法需要body）
                if is_body_parameter(handler_args, index, inputs.len() - 2) {
                    body_type = Some(param_type);
                    body_method_call = RequestBuilder::generate_body_method_call(
                        &handler_args.content_type,
                        &handler_args.method,
                    );
                }
            }
        }
    }

    let params = if param_tokens.is_empty() {
        quote! {}
    } else {
        quote! { #(#param_tokens)* }
    };

    (
        body_type.map(|t| quote! { #t }).unwrap_or(quote! {}),
        params,
        body_method_call,
    )
}

/// 判断是否为body参数
fn is_body_parameter(handler_args: &HandlerArgs, param_index: usize, total_params: usize) -> bool {
    // 如果方法需要body且这是最后一个参数
    match handler_args.method {
        swan_common::HttpMethod::Post | swan_common::HttpMethod::Put => {
            // POST/PUT 方法的最后一个参数通常是body
            total_params > 0 && param_index == total_params - 1 && handler_args.content_type.is_some()
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_validate_function_inputs_valid() {
        let mut inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]> = syn::punctuated::Punctuated::new();
        inputs.push(parse_quote! { &self });
        let result = validate_function_inputs(&inputs);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_function_inputs_empty() {
        let inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]> = syn::punctuated::Punctuated::new();
        let result = validate_function_inputs(&inputs);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_function_inputs_no_self() {
        let mut inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]> = syn::punctuated::Punctuated::new();
        inputs.push(parse_quote! { param: String });
        let result = validate_function_inputs(&inputs);
        assert!(result.is_err());
    }
}