use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{LitStr, Meta, Path, Token};
use crate::types::{ContentType, HandlerArgs, HttpMethod, RetryConfig};

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let method = None;
        let mut url = None;
        let mut content_type = None;
        let mut headers = Punctuated::new();
        let mut interceptor = None;
        let mut retry = None;

        let pairs = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for pair in pairs {
            if let Meta::NameValue(name_value) = pair {
                let key = name_value.path.get_ident().ok_or_else(|| {
                    syn::Error::new_spanned(&name_value.path, "expected identifier as key")
                })?;

                match key.to_string().as_str() {
                    "url" => {
                        url = Some(parse_url_value(&name_value.value)?);
                    }
                    "content_type" => {
                        content_type = Some(parse_content_type_value(&name_value.value)?);
                    }
                    "header" => {
                        headers.push(parse_header_value(&name_value.value)?);
                    }
                    "interceptor" => {
                        interceptor = Some(parse_interceptor_value(&name_value.value)?);
                    }
                    "retry" => {
                        retry = Some(parse_retry_value(&name_value.value)?);
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            key,
                            "Only 'url', 'content_type', 'header', 'interceptor', and 'retry' are supported",
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(pair, "expected key-value pair"));
            }
        }

        let url = url.ok_or_else(|| syn::Error::new(input.span(), "Missing required 'url' parameter"))?;
        let method = method.unwrap_or(HttpMethod::Get);
        
        Ok(HandlerArgs {
            method,
            url,
            content_type,
            headers,
            interceptor,
            retry,
        })
    }
}

fn parse_url_value(value: &syn::Expr) -> syn::Result<LitStr> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = value
    {
        Ok(lit.clone())
    } else {
        Err(syn::Error::new_spanned(
            value,
            "url must be a string literal",
        ))
    }
}

fn parse_content_type_value(value: &syn::Expr) -> syn::Result<ContentType> {
    if let syn::Expr::Path(expr_path) = value {
        let ident = expr_path.path.get_ident().ok_or_else(|| {
            syn::Error::new_spanned(
                &expr_path,
                "content_type must be a simple identifier",
            )
        })?;
        match ident.to_string().as_str() {
            "json" => Ok(ContentType::Json),
            "form_urlencoded" => Ok(ContentType::FormUrlEncoded),
            "form_multipart" => Ok(ContentType::FormMultipart),
            _ => {
                Err(syn::Error::new_spanned(
                    ident,
                    "content_type must be one of 'json', 'form_urlencoded', or 'form_multipart'",
                ))
            }
        }
    } else {
        Err(syn::Error::new_spanned(
            value,
            "content_type must be an identifier (e.g., json, form_urlencoded, or form_multipart)",
        ))
    }
}

fn parse_header_value(value: &syn::Expr) -> syn::Result<LitStr> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = value
    {
        Ok(lit.clone())
    } else {
        Err(syn::Error::new_spanned(
            value,
            "header must be a string literal",
        ))
    }
}

fn parse_interceptor_value(value: &syn::Expr) -> syn::Result<Path> {
    if let syn::Expr::Path(expr_path) = value {
        Ok(expr_path.path.clone())
    } else {
        Err(syn::Error::new_spanned(
            value,
            "interceptor must be a trait path",
        ))
    }
}

fn parse_retry_value(value: &syn::Expr) -> syn::Result<RetryConfig> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = value
    {
        RetryConfig::parse(lit)
    } else {
        Err(syn::Error::new_spanned(
            value,
            "retry must be a string literal (e.g., \"exponential(3, 100ms)\")",
        ))
    }
}

/// 解析处理器参数的公共函数
pub fn parse_handler_args(input: ParseStream) -> syn::Result<HandlerArgs> {
    HandlerArgs::parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_url_value() {
        let expr = parse_quote! { "/api/test" };
        let result = parse_url_value(&expr).unwrap();
        assert_eq!(result.value(), "/api/test");
    }

    #[test]
    fn test_parse_content_type_value() {
        let expr = parse_quote! { json };
        let result = parse_content_type_value(&expr).unwrap();
        assert_eq!(result, ContentType::Json);
    }

    #[test]
    fn test_parse_header_value() {
        let expr = parse_quote! { "Authorization: Bearer token" };
        let result = parse_header_value(&expr).unwrap();
        assert_eq!(result.value(), "Authorization: Bearer token");
    }
}