use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};

/// HTTP 方法枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    /// 返回HTTP方法的字符串表示
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }

    /// 返回用于客户端的方法标识符
    pub fn client_method(&self) -> Ident {
        Ident::new(
            match self {
                HttpMethod::Get => "get",
                HttpMethod::Post => "post",
                HttpMethod::Put => "put",
                HttpMethod::Delete => "delete",
            },
            proc_macro2::Span::call_site(),
        )
    }
}

/// 内容类型枚举
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Json,
    FormUrlEncoded,
    FormMultipart,
}

impl Parse for ContentType {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: Ident = input.parse()?;
        match ident.to_string().as_str() {
            "json" => Ok(ContentType::Json),
            "form_urlencoded" => Ok(ContentType::FormUrlEncoded),
            "form_multipart" => Ok(ContentType::FormMultipart),
            _ => Err(syn::Error::new_spanned(
                ident,
                "content_type must be one of 'json', 'form_urlencoded', or 'form_multipart'",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_http_method_client_method() {
        assert_eq!(HttpMethod::Get.client_method().to_string(), "get");
        assert_eq!(HttpMethod::Post.client_method().to_string(), "post");
        assert_eq!(HttpMethod::Put.client_method().to_string(), "put");
        assert_eq!(HttpMethod::Delete.client_method().to_string(), "delete");
    }
}