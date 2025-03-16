use async_trait::async_trait;
use proc_macro2::Ident;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{LitStr, Meta, Path, Token};

#[derive(Debug)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
        }
    }

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

#[derive(Debug)]
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

pub struct HandlerArgs {
    pub url: LitStr,
    pub method: HttpMethod,
    pub content_type: Option<ContentType>,
    pub headers: Punctuated<LitStr, Token![,]>,
    pub interceptor: Option<Path>,
}

impl Parse for HandlerArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut method = None;
        let mut url = None;
        let mut content_type = None;
        let mut headers = Punctuated::new();
        let mut interceptor = None;

        let pairs = Punctuated::<Meta, Token![,]>::parse_terminated(input)?;
        for pair in pairs {
            if let Meta::NameValue(name_value) = pair {
                let key = name_value.path.get_ident().ok_or_else(|| {
                    syn::Error::new_spanned(&name_value.path, "expected identifier as key")
                })?;

                match key.to_string().as_str() {
                    "url" => {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }) = name_value.value
                        {
                            url = Some(lit);
                        } else {
                            return Err(syn::Error::new_spanned(
                                &name_value.value,
                                "url must be a string literal",
                            ));
                        }
                    }
                    "content_type" => {
                        if let syn::Expr::Path(expr_path) = name_value.value {
                            let ident = expr_path.path.get_ident().ok_or_else(|| {
                                syn::Error::new_spanned(
                                    &expr_path,
                                    "content_type must be a simple identifier",
                                )
                            })?;
                            content_type = Some(match ident.to_string().as_str() {
                                "json" => ContentType::Json,
                                "form_urlencoded" => ContentType::FormUrlEncoded,
                                "form_multipart" => ContentType::FormMultipart,
                                _ => {
                                    return Err(syn::Error::new_spanned(
                                        ident,
                                        "content_type must be one of 'json', 'form_urlencoded', or 'form_multipart'",
                                    ));
                                }
                            });
                        } else {
                            return Err(syn::Error::new_spanned(
                                &name_value.value,
                                "content_type must be an identifier (e.g., json, form_urlencoded, or form_multipart)",
                            ));
                        }
                    }
                    "header" => {
                        if let syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }) = name_value.value
                        {
                            headers.push(lit);
                        } else {
                            return Err(syn::Error::new_spanned(
                                &name_value.value,
                                "header must be a string literal",
                            ));
                        }
                    }
                    "interceptor" => {
                        // 新增 interceptor 的解析
                        if let syn::Expr::Path(expr_path) = name_value.value {
                            interceptor = Some(expr_path.path);
                        } else {
                            return Err(syn::Error::new_spanned(
                                &name_value.value,
                                "interceptor must be a trait path",
                            ));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            key,
                            "Only 'url' and 'content_type' are supported",
                        ));
                    }
                }
            } else {
                return Err(syn::Error::new_spanned(pair, "expected key-value pair"));
            }
        }

        // 确保 url 存在
        let url =
            url.ok_or_else(|| syn::Error::new(input.span(), "Missing required 'url' parameter"))?;

        // 默认为get
        let method = method.unwrap_or(HttpMethod::Get);
        Ok(HandlerArgs {
            method,
            url,
            content_type,
            headers,
            interceptor,
        })
    }
}

pub fn parse_handler_args(input: ParseStream) -> syn::Result<HandlerArgs> {
    HandlerArgs::parse(input)
}

#[async_trait]
pub trait SwanInterceptor {
    async fn before_request(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &Vec<u8>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Vec<u8>)>;
    async fn after_response(
        &self,
        response: reqwest::Response,
    ) -> anyhow::Result<reqwest::Response>;
}

pub struct HttpClientArgs {
    pub base_url: Option<LitStr>,
    pub interceptor: Option<Path>,
}

impl Parse for HttpClientArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut base_url = None;
        let mut interceptor = None;

        // 解析 key = value, key = value, ...
        let pairs = Punctuated::<syn::Meta, Token![,]>::parse_terminated(input)?;
        for meta in pairs {
            if let syn::Meta::NameValue(nv) = meta {
                if nv.path.is_ident("base_url") {
                    if let syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(lit),
                        ..
                    }) = nv.value
                    {
                        base_url = Some(lit);
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.value,
                            "base_url must be a string literal",
                        ));
                    }
                } else if nv.path.is_ident("interceptor") {
                    if let syn::Expr::Path(expr_path) = nv.value {
                        interceptor = Some(expr_path.path);
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.value,
                            "interceptor must be a trait path",
                        ));
                    }
                } else {
                    return Err(syn::Error::new_spanned(
                        nv.path,
                        "Only 'base_url' or 'interceptor' are supported",
                    ));
                }
            } else {
                return Err(syn::Error::new_spanned(meta, "Expected key-value pair"));
            }
        }

        Ok(HttpClientArgs {
            base_url,
            interceptor,
        })
    }
}

// 定义一个解析器函数，供 parse_macro_input! 使用
pub fn parse_http_client_args(input: ParseStream) -> syn::Result<HttpClientArgs> {
    HttpClientArgs::parse(input)
}
