use proc_macro::TokenStream;
use quote::quote;
use swan_common::{ContentType, HandlerArgs, HttpMethod};
use syn::{FnArg, GenericArgument, LitStr, PathArguments, ReturnType, Signature, Type};

pub fn generate_http_method(fn_sig: &Signature, handler_args: &HandlerArgs) -> TokenStream {
    let fn_name = &fn_sig.ident;
    let inputs = &fn_sig.inputs;
    let output = &fn_sig.output;

    let url = handler_args.url.value();
    let method = &handler_args.method;
    let content_type = &handler_args.content_type;
    let headers = &handler_args.headers;
    let method_interceptor = &handler_args.interceptor;

    let method_interceptor_init = method_interceptor.clone()
        .map(|path| quote! { Some(Box::new(<#path as Default>::default()) as Box<dyn swan_common::SwanInterceptor>) })
        .unwrap_or(quote! { None });

    if inputs.is_empty() {
        return syn::Error::new_spanned(inputs, "method must have at least 'self' parameter")
            .to_compile_error()
            .into();
    }

    let self_arg = inputs.iter().nth(0).unwrap();
    if !matches!(self_arg, FnArg::Receiver(_)) {
        return syn::Error::new_spanned(
            self_arg,
            "first parameter must be 'self', '&self', or '&mut self'",
        )
        .to_compile_error()
        .into();
    }

    let (body_type, body_param, body_method_call) = if inputs.len() == 2 {
        let body_arg = inputs.iter().nth(1).unwrap();
        let body_type = match body_arg {
            FnArg::Typed(pat_type) => &pat_type.ty,
            _ => {
                return syn::Error::new_spanned(body_arg, "second parameter must be typed")
                    .to_compile_error()
                    .into();
            }
        };
        let method_call = match content_type {
            Some(ContentType::Json) => quote! { .json(&body) },
            Some(ContentType::FormUrlEncoded) => quote! { .form(&body) },
            Some(ContentType::FormMultipart) => quote! { .multipart(&body) },
            None => quote! { .query(&body) }, // 默认使用 query 参数
        };
        // 仅对 POST 和 PUT 使用 body
        let method_call = match method {
            HttpMethod::Post | HttpMethod::Put => method_call,
            HttpMethod::Get | HttpMethod::Delete => quote! { .query(&body) },
        };
        (
            quote! { #body_type },
            quote! { , body: #body_type },
            method_call,
        )
    } else {
        (quote! {}, quote! {}, quote! {})
    };

    let (ok_type, err_type) = match output {
        ReturnType::Type(_, ty) => {
            // 确保返回类型是一个路径（例如 anyhow::Result<T>）
            let type_path = match &**ty {
                Type::Path(type_path) => type_path,
                _ => {
                    return syn::Error::new_spanned(ty, "Return type must be anyhow::Result<T>")
                        .to_compile_error()
                        .into();
                }
            };

            // 检查路径是否以 "anyhow" 开头
            let first_segment = if let Some(segment) = type_path.path.segments.first() {
                segment
            } else {
                return syn::Error::new_spanned(ty, "Return type path must not be empty")
                    .to_compile_error()
                    .into();
            };

            if first_segment.ident != "anyhow" {
                return syn::Error::new_spanned(ty, "Return type must be anyhow::Result<T>")
                    .to_compile_error()
                    .into();
            }

            // 检查路径是否以 "Result" 结尾
            let last_segment = if let Some(segment) = type_path.path.segments.last() {
                segment
            } else {
                return syn::Error::new_spanned(ty, "Return type path must not be empty")
                    .to_compile_error()
                    .into();
            };

            if last_segment.ident != "Result" {
                return syn::Error::new_spanned(ty, "Return type must be anyhow::Result<T>")
                    .to_compile_error()
                    .into();
            }

            // 提取泛型参数
            let args = match &last_segment.arguments {
                PathArguments::AngleBracketed(args) => args,
                _ => {
                    return syn::Error::new_spanned(
                        ty,
                        "anyhow::Result<T> must have generic arguments",
                    )
                    .to_compile_error()
                    .into();
                }
            };

            // 确保泛型参数数量为 1（anyhow::Result<T>）
            if args.args.len() != 1 {
                return syn::Error::new_spanned(
                    ty,
                    "anyhow::Result<T> must have exactly 1 type parameter",
                )
                .to_compile_error()
                .into();
            }

            // 提取 Ok 类型，Err 类型固定为 anyhow::Error
            let ok_type = &args.args[0];
            (ok_type, quote! { anyhow::Error })
        }
        _ => {
            return syn::Error::new_spanned(output, "Function must return anyhow::Result<T>")
                .to_compile_error()
                .into();
        }
    };

    let method_ident = method.client_method();

    // 在编译时处理 headers
    let header_statements = headers.iter().map(|header| {
        let header_str = header.value();
        // 在编译时分割 header
        let parts: Vec<&str> = header_str.splitn(2, ": ").collect();
        if parts.len() == 2 {
            let key = LitStr::new(parts[0], header.span());
            let value = LitStr::new(parts[1], header.span());
            quote! {
                .header(#key, #value)
            }
        } else {
            // 如果 header 格式不正确，在编译时产生错误
            let error = syn::Error::new(
                header.span(),
                "header must be in 'Key: Value' format with a colon and space separator",
            );
            error.to_compile_error()
        }
    });

    let content_type_header = match content_type {
        Some(ContentType::Json) => quote! { .header("Content-Type", "application/json") },
        Some(ContentType::FormUrlEncoded) => {
            quote! { .header("Content-Type", "application/x-www-form-urlencoded") }
        }
        Some(ContentType::FormMultipart) => {
            quote! { .header("Content-Type", "multipart/form-data") }
        }
        None => quote! { .header("Content-Type", "application/json") },
    };

    let type_conversion = generate_type_conversion(ok_type);

    let expanded = quote! {
        pub async fn #fn_name(&self #body_param) #output {

            let method_interceptor = #method_interceptor_init;

            let full_url = format!("{}{}", self.base_url, #url);

            let mut request_builder = self.client
                .#method_ident(&full_url)
                #content_type_header
                #(#header_statements)*
                #body_method_call;

            let request_body = serde_json::to_vec(&body)?;

            let (mut modified_request_builder,request_body) = match &self.interceptor {
                None => (request_builder,request_body),
                Some(interceptor) => match interceptor.before_request(request_builder,&request_body).await {
                    Ok((modified_request_builder,modified_request_body)) => (modified_request_builder,modified_request_body),
                    Err(e) => return Err(anyhow::anyhow!("Global Interceptor before_request failed: {}", e)),
                },
            };

            let (mut modified_request_builder, request_body) = match &method_interceptor {
            None => (modified_request_builder, request_body),
            Some(interceptor) => match interceptor.before_request(modified_request_builder, &request_body).await {
                Ok((modified_request_builder, modified_request_body)) => (modified_request_builder, modified_request_body),
                Err(e) => return Err(anyhow::anyhow!("Method interceptor before_request failed: {}", e)),
            },
        };

            let request_body = serde_json::from_slice::<Value>(request_body.as_slice())?;
            log::info!("Original Request:{}",request_body);
            let request = match modified_request_builder.build() {
                Ok(req) => req,
                Err(e) => return Err(anyhow::anyhow!("Failed to build request: {}", e)),
            };

            let response = match self.client.execute(request).await {
                Ok(resp) => resp,
                Err(e) => return Err(anyhow::anyhow!("Request failed: {}", e)),
            };

            let response = match &method_interceptor {
                None => response,
                Some(interceptor) => match interceptor.after_response(response).await {
                    Ok(resp) => resp,
                    Err(e) => return Err(anyhow::anyhow!("Method interceptor after_response failed: {}", e)),
                },
            };

            let response = match &self.interceptor {
                None => response,
                Some(interceptor) => match interceptor.after_response(response).await {
                    Ok(resp) => resp,
                    Err(e) => return Err(anyhow::anyhow!("Global Interceptor after_response failed: {}", e)),
                },
            };

            if response.status().is_success() {
                let bytes = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(e) => return Err(anyhow::anyhow!("Failed to read response bytes: {}", e)),
                };

                let result = #type_conversion;
                log::info!("Original Response:{:?}",result);
                Ok(result)
            } else {
                Err(anyhow::anyhow!("Request failed with status: {}", response.status()))
            }
        }
    };

    TokenStream::from(expanded)
}

fn generate_type_conversion(ok_type: &GenericArgument) -> proc_macro2::TokenStream {
    let default_conversion = quote! {
        serde_json::from_slice::<#ok_type>(&bytes)?
    };

    match ok_type {
        GenericArgument::Type(Type::Path(type_path)) => {
            let last_segment = type_path.path.segments.last().unwrap();
            match last_segment.ident.to_string().as_str() {
                "String" => quote! {
                   String::from_utf8_lossy(&bytes).to_string()
                },
                "Vec" => match &last_segment.arguments {
                    PathArguments::AngleBracketed(args) => match args.args.first() {
                        Some(GenericArgument::Type(Type::Path(inner_type)))
                            if inner_type.path.segments.last().unwrap().ident == "u8" =>
                        {
                            quote! {
                                bytes.to_vec()
                            }
                        }
                        _ => default_conversion,
                    },
                    _ => default_conversion,
                },
                _ => default_conversion,
            }
        }
        _ => default_conversion,
    }
}
