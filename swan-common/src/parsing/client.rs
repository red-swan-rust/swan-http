use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{LitStr, Path, Token};
use crate::types::{HttpClientArgs, ProxyConfig, ProxyType};

impl Parse for HttpClientArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut base_url = None;
        let mut interceptor = None;
        let mut state = None;
        let mut proxy = None;

        let pairs = Punctuated::<syn::Meta, Token![,]>::parse_terminated(input)?;
        for meta in pairs {
            match meta {
                syn::Meta::NameValue(nv) => {
                    if nv.path.is_ident("base_url") {
                        base_url = Some(parse_base_url_value(&nv.value)?);
                    } else if nv.path.is_ident("interceptor") {
                        interceptor = Some(parse_interceptor_value(&nv.value)?);
                    } else if nv.path.is_ident("state") {
                        state = Some(parse_state_value(&nv.value)?);
                    } else if nv.path.is_ident("proxy") {
                        proxy = Some(parse_proxy_simple_value(&nv.value)?);
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv.path,
                            "Only 'base_url', 'interceptor', 'state', or 'proxy' are supported",
                        ));
                    }
                }
                syn::Meta::List(ml) if ml.path.is_ident("proxy") => {
                    proxy = Some(parse_proxy_full_value(&ml)?);
                }
                _ => {
                    return Err(syn::Error::new_spanned(meta, "Expected key-value pair or function-like macro"));
                }
            }
        }

        // 验证：如果使用了 state，必须同时提供 interceptor
        if state.is_some() && interceptor.is_none() {
            return Err(syn::Error::new(
                input.span(),
                "When using 'state', 'interceptor' must also be provided"
            ));
        }

        Ok(HttpClientArgs {
            base_url,
            interceptor,
            state,
            proxy,
        })
    }
}

fn parse_base_url_value(value: &syn::Expr) -> syn::Result<LitStr> {
    if let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(lit),
        ..
    }) = value
    {
        Ok(lit.clone())
    } else {
        Err(syn::Error::new_spanned(
            value,
            "base_url must be a string literal",
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

fn parse_state_value(value: &syn::Expr) -> syn::Result<Path> {
    if let syn::Expr::Path(expr_path) = value {
        Ok(expr_path.path.clone())
    } else {
        Err(syn::Error::new_spanned(
            value,
            "state must be a type path",
        ))
    }
}

fn parse_proxy_simple_value(value: &syn::Expr) -> syn::Result<ProxyConfig> {
    match value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(lit),
            ..
        }) => Ok(ProxyConfig::Simple(lit.clone())),
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Bool(lit),
            ..
        }) => {
            if lit.value {
                Err(syn::Error::new_spanned(
                    value,
                    "proxy = true is not supported, use proxy = \"url\" instead",
                ))
            } else {
                Ok(ProxyConfig::Disabled(lit.clone()))
            }
        }
        _ => Err(syn::Error::new_spanned(
            value,
            "proxy must be a string literal (URL) or false (to disable)",
        ))
    }
}

fn parse_proxy_full_value(meta_list: &syn::MetaList) -> syn::Result<ProxyConfig> {
    let mut proxy_type = None;
    let mut url = None;
    let mut username = None;
    let mut password = None;
    let mut no_proxy = None;

    let nested = meta_list.parse_args_with(Punctuated::<syn::Meta, Token![,]>::parse_terminated)?;
    
    for meta in nested {
        if let syn::Meta::NameValue(nv) = meta {
            if nv.path.is_ident("type") {
                if let syn::Expr::Path(expr_path) = &nv.value {
                    if let Some(ident) = expr_path.path.get_ident() {
                        let type_str = ident.to_string();
                        proxy_type = ProxyType::from_str(&type_str);
                        if proxy_type.is_none() {
                            return Err(syn::Error::new_spanned(
                                &nv.value, 
                                "proxy type must be 'http' or 'socks5'"
                            ));
                        }
                    } else {
                        return Err(syn::Error::new_spanned(&nv.value, "proxy type must be an identifier"));
                    }
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "proxy type must be an identifier"));
                }
            } else if nv.path.is_ident("url") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value {
                    url = Some(lit.clone());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "url must be a string literal"));
                }
            } else if nv.path.is_ident("username") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value {
                    username = Some(lit.clone());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "username must be a string literal"));
                }
            } else if nv.path.is_ident("password") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value {
                    password = Some(lit.clone());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "password must be a string literal"));
                }
            } else if nv.path.is_ident("no_proxy") {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = &nv.value {
                    no_proxy = Some(lit.clone());
                } else {
                    return Err(syn::Error::new_spanned(&nv.value, "no_proxy must be a string literal"));
                }
            } else {
                return Err(syn::Error::new_spanned(
                    &nv.path,
                    "Only 'type', 'url', 'username', 'password', or 'no_proxy' are supported in proxy configuration",
                ));
            }
        } else {
            return Err(syn::Error::new_spanned(meta, "Expected key-value pair in proxy configuration"));
        }
    }

    let url = url.ok_or_else(|| {
        syn::Error::new_spanned(&meta_list.path, "proxy configuration must include 'url'")
    })?;

    Ok(ProxyConfig::Full {
        proxy_type,
        url,
        username,
        password,
        no_proxy,
    })
}

/// 解析HTTP客户端参数的公共函数
pub fn parse_http_client_args(input: ParseStream) -> syn::Result<HttpClientArgs> {
    HttpClientArgs::parse(input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;
    use quote::quote;

    #[test]
    fn test_parse_base_url_value() {
        let expr = parse_quote! { "https://api.example.com" };
        let result = parse_base_url_value(&expr).unwrap();
        assert_eq!(result.value(), "https://api.example.com");
    }

    #[test]
    fn test_parse_interceptor_value() {
        let expr = parse_quote! { MyInterceptor };
        let result = parse_interceptor_value(&expr).unwrap();
        assert_eq!(result.segments.len(), 1);
        assert_eq!(result.segments.first().unwrap().ident.to_string(), "MyInterceptor");
    }

    #[test]
    fn test_invalid_base_url() {
        let expr = parse_quote! { 123 };
        let result = parse_base_url_value(&expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_state_without_interceptor_should_fail() {
        let tokens = quote! { state = MyState };
        let result = syn::parse2::<HttpClientArgs>(tokens);
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.to_string().contains("When using 'state', 'interceptor' must also be provided"));
        }
    }

    #[test]
    fn test_state_with_interceptor_should_succeed() {
        let tokens = quote! { interceptor = MyInterceptor, state = MyState };
        let result = syn::parse2::<HttpClientArgs>(tokens);
        assert!(result.is_ok());
    }
}