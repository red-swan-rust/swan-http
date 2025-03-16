mod common;
mod generate;

use crate::common::common_http_method;
use proc_macro::TokenStream;
use quote::quote;
use swan_common::{HttpMethod, parse_http_client_args};
use syn::{ItemStruct, parse_macro_input};

#[proc_macro_attribute]
pub fn http_client(args: TokenStream, item: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(item as ItemStruct);
    let struct_name = &input.ident;

    // 确保结构体是空的（无字段）
    if !matches!(input.fields, syn::Fields::Unit) {
        return syn::Error::new_spanned(
            &input,
            "http_client macro only supports empty structs (e.g., `struct ApiClient;`)",
        )
        .to_compile_error()
        .into();
    }

    // 解析属性参数
    let args = parse_macro_input!(args with parse_http_client_args);
    let base_url = args.base_url.map(|lit| lit.value()).unwrap_or_default();
    let interceptor = args.interceptor;

    // 添加必要的字段
    input.fields = syn::Fields::Named(syn::parse_quote! {{
        client: reqwest::Client,
        base_url: String,
        interceptor: Option<Box<dyn swan_common::SwanInterceptor>>,
    }});

    let interceptor_init = interceptor
        .map(|path| quote! { Some(Box::new(<#path as Default>::default()) as Box<dyn swan_common::SwanInterceptor>) })
        .unwrap_or(quote! { None });

    let expanded = quote! {
        #input

        impl #struct_name {
            pub fn new() -> Self {
                #struct_name {
                    client: reqwest::Client::new(),
                    base_url: #base_url.to_string(),
                    interceptor: #interceptor_init,
                }
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn post(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Post)
}

#[proc_macro_attribute]
pub fn get(args: TokenStream, item: TokenStream) -> TokenStream {
    common_http_method(args, item, HttpMethod::Get)
}
