// Swan HTTP Client Library - Procedural Macros
//
// This module provides the procedural macros for the Swan HTTP client library.
// It includes macros for defining HTTP clients and HTTP method handlers.

mod common;
mod generator;
mod conversion;
mod request;
mod error;
mod optimization;

use crate::common::common_http_method;
use crate::generator::generate_http_client_impl;
use proc_macro::TokenStream;
use swan_common::{HttpMethod, parse_http_client_args};
use syn::{ItemStruct, parse_macro_input};

/// HTTP 客户端宏
/// 
/// 用于为空结构体生成 HTTP 客户端实现。
/// 
/// # 参数
/// 
/// * `base_url` - 可选的基础 URL
/// * `interceptor` - 可选的全局拦截器
/// 
/// # 示例
/// 
/// ```rust
/// use swan_macro::http_client;
/// 
/// #[http_client(base_url = "https://api.example.com")]
/// struct ApiClient;
/// ```
#[proc_macro_attribute]
pub fn http_client(args: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemStruct);
    let args = parse_macro_input!(args with parse_http_client_args);

    match generate_http_client_impl(input, &args) {
        Ok(tokens) => tokens,
        Err(error) => error.to_compile_error().into(),
    }
}

/// POST 方法宏
/// 
/// 用于为方法生成 POST 请求实现。
/// 
/// # 参数
/// 
/// * `url` - 请求 URL（相对于客户端基础 URL）
/// * `content_type` - 可选的内容类型
/// * `header` - 可选的额外头部
/// * `interceptor` - 可选的方法级拦截器
/// 
/// # 示例
/// 
/// ```rust
/// use swan_macro::post;
/// use serde::{Deserialize, Serialize};
/// 
/// #[derive(Serialize)]
/// struct CreateUserRequest {
///     name: String,
///     email: String,
/// }
/// 
/// #[derive(Deserialize)]
/// struct User {
///     id: u32,
///     name: String,
///     email: String,
/// }
/// 
/// impl ApiClient {
///     #[post(url = "/users", content_type = json)]
///     async fn create_user(&self, body: CreateUserRequest) -> anyhow::Result<User> {}
/// }
/// ```
#[proc_macro_attribute]
pub fn post(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Post)
}

/// GET 方法宏
/// 
/// 用于为方法生成 GET 请求实现。
/// 
/// # 参数
/// 
/// * `url` - 请求 URL（相对于客户端基础 URL）
/// * `header` - 可选的额外头部
/// * `interceptor` - 可选的方法级拦截器
/// 
/// # 示例
/// 
/// ```rust
/// use swan_macro::get;
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct User {
///     id: u32,
///     name: String,
///     email: String,
/// }
/// 
/// impl ApiClient {
///     #[get(url = "/users/1")]
///     async fn get_user(&self) -> anyhow::Result<User> {}
/// }
/// ```
#[proc_macro_attribute]
pub fn get(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Get)
}

/// PUT 方法宏
/// 
/// 用于为方法生成 PUT 请求实现。
#[proc_macro_attribute]
pub fn put(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Put)
}

/// DELETE 方法宏
/// 
/// 用于为方法生成 DELETE 请求实现。
#[proc_macro_attribute]
pub fn delete(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Delete)
}
