// Swan HTTP Client Library - Common Types and Utilities
//
// This module provides the core types, traits, and utilities used by the Swan HTTP client library.
// It is organized into several sub-modules for better maintainability and separation of concerns.

pub mod types;
pub mod parsing;
pub mod interceptor;

// Re-export commonly used types and traits for convenience
pub use types::{HttpMethod, ContentType, HandlerArgs, HttpClientArgs, RetryPolicy, RetryConfig};
pub use parsing::{parse_handler_args, parse_http_client_args};
pub use interceptor::{SwanInterceptor, InterceptorCache, NoOpInterceptor, ClientStateMarker};

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // 测试所有公共API都能正确导入
        let _method = HttpMethod::Get;
        let _content_type = ContentType::Json;
        let _noop = NoOpInterceptor::default();
        
        // 确保解析函数可用
        use syn::parse::ParseStream;
        let _parse_fn: fn(ParseStream) -> syn::Result<HandlerArgs> = parse_handler_args;
        let _parse_client_fn: fn(ParseStream) -> syn::Result<HttpClientArgs> = parse_http_client_args;
    }
}