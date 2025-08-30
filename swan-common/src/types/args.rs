use syn::punctuated::Punctuated;
use syn::{LitStr, Path, Token};
use crate::types::http::{HttpMethod, ContentType};
use crate::types::retry::RetryConfig;

/// HTTP 处理器参数配置
pub struct HandlerArgs {
    pub url: LitStr,
    pub method: HttpMethod,
    pub content_type: Option<ContentType>,
    pub headers: Punctuated<LitStr, Token![,]>,
    pub interceptor: Option<Path>,
    pub retry: Option<RetryConfig>,
}

/// HTTP 客户端参数配置
pub struct HttpClientArgs {
    pub base_url: Option<LitStr>,
    pub interceptor: Option<Path>,
    pub state: Option<Path>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::LitStr;
    use proc_macro2::Span;

    #[test]
    fn test_handler_args_creation() {
        let url = LitStr::new("/test", Span::call_site());
        let headers = Punctuated::new();
        
        let args = HandlerArgs {
            url,
            method: HttpMethod::Get,
            content_type: Some(ContentType::Json),
            headers,
            interceptor: None,
            retry: None,
        };

        assert_eq!(args.method, HttpMethod::Get);
        assert_eq!(args.url.value(), "/test");
        assert_eq!(args.content_type, Some(ContentType::Json));
    }

    #[test]
    fn test_http_client_args_creation() {
        let base_url = Some(LitStr::new("https://api.example.com", Span::call_site()));
        
        let args = HttpClientArgs {
            base_url,
            interceptor: None,
            state: None,
        };

        assert!(args.base_url.is_some());
        assert_eq!(args.base_url.unwrap().value(), "https://api.example.com");
    }
}