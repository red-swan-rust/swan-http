use quote::quote;
use syn::{LitStr, FnArg};
use swan_common::{ContentType, HandlerArgs, HttpMethod};
use super::DynamicParamsProcessor;

/// 请求构建器
/// 
/// 负责生成 HTTP 请求构建的代码，包括 URL 拼接、头部设置、请求体处理等。
pub struct RequestBuilder;

impl RequestBuilder {
    /// 生成请求构建代码
    /// 
    /// # 参数
    /// 
    /// * `handler_args` - 处理器参数
    /// * `body_method_call` - 请求体方法调用代码
    /// * `fn_inputs` - 函数参数列表（用于动态参数替换）
    /// 
    /// # 返回值
    /// 
    /// 生成的请求构建代码
    pub fn generate_request_builder_code(
        handler_args: &HandlerArgs,
        body_method_call: &proc_macro2::TokenStream,
        fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    ) -> proc_macro2::TokenStream {
        let url = handler_args.url.value();
        let method = &handler_args.method;
        let headers = &handler_args.headers;

        let method_ident = method.client_method();
        
        // 生成动态URL代码
        let url_code = DynamicParamsProcessor::generate_dynamic_url_code(&url, fn_inputs);
        
        // 生成动态header代码
        let header_statements = Self::generate_dynamic_header_statements(headers, fn_inputs);
        let content_type_header = Self::generate_content_type_header(&handler_args.content_type);

        // 如果没有 body_method_call，则不添加它
        let body_call = if body_method_call.is_empty() {
            quote! {}
        } else {
            body_method_call.clone()
        };

        quote! {
            #url_code

            let request_builder = self.client
                .#method_ident(&full_url)
                #content_type_header
                #(#header_statements)*
                #body_call;
        }
    }


    /// 生成动态头部设置代码（支持参数占位符）
    fn generate_dynamic_header_statements(
        headers: &syn::punctuated::Punctuated<LitStr, syn::Token![,]>,
        fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    ) -> Vec<proc_macro2::TokenStream> {
        headers.iter().map(|header| {
            let header_str = header.value();
            DynamicParamsProcessor::generate_dynamic_header_code(&header_str, fn_inputs)
        }).collect()
    }

    /// 生成内容类型头部代码
    fn generate_content_type_header(content_type: &Option<ContentType>) -> proc_macro2::TokenStream {
        match content_type {
            Some(ContentType::Json) => quote! { .header("Content-Type", "application/json") },
            Some(ContentType::FormUrlEncoded) => {
                quote! { .header("Content-Type", "application/x-www-form-urlencoded") }
            }
            Some(ContentType::FormMultipart) => {
                quote! { .header("Content-Type", "multipart/form-data") }
            }
            None => quote! { .header("Content-Type", "application/json") },
        }
    }

    /// 生成请求体方法调用代码
    /// 
    /// # 参数
    /// 
    /// * `content_type` - 内容类型
    /// * `method` - HTTP 方法
    /// 
    /// # 返回值
    /// 
    /// 生成的请求体方法调用代码
    pub fn generate_body_method_call(
        content_type: &Option<ContentType>,
        method: &HttpMethod,
    ) -> proc_macro2::TokenStream {
        let method_call = match content_type {
            Some(ContentType::Json) => quote! { .json(&body) },
            Some(ContentType::FormUrlEncoded) => quote! { .form(&body) },
            Some(ContentType::FormMultipart) => quote! { .multipart(&body) },
            None => quote! { .query(&body) },
        };

        // 仅对 POST 和 PUT 使用 body，GET 和 DELETE 使用 query 参数
        match method {
            HttpMethod::Post | HttpMethod::Put => method_call,
            HttpMethod::Get | HttpMethod::Delete => quote! { .query(&body) },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_content_type_header_json() {
        let content_type = Some(ContentType::Json);
        let result = RequestBuilder::generate_content_type_header(&content_type);
        let expected = quote! { .header("Content-Type", "application/json") };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_body_method_call_post_json() {
        let content_type = Some(ContentType::Json);
        let method = HttpMethod::Post;
        let result = RequestBuilder::generate_body_method_call(&content_type, &method);
        let expected = quote! { .json(&body) };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_generate_body_method_call_get_with_json() {
        let content_type = Some(ContentType::Json);
        let method = HttpMethod::Get;
        let result = RequestBuilder::generate_body_method_call(&content_type, &method);
        let expected = quote! { .query(&body) };
        assert_eq!(result.to_string(), expected.to_string());
    }
}