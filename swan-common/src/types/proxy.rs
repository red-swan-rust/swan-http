use syn::{LitStr, LitBool};

/// 代理类型枚举
#[derive(Debug, Clone, PartialEq)]
pub enum ProxyType {
    /// HTTP/HTTPS 代理
    Http,
    /// SOCKS5 代理
    Socks5,
}

impl ProxyType {
    /// 从字符串解析代理类型
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "http" => Some(ProxyType::Http),
            "socks5" => Some(ProxyType::Socks5),
            _ => None,
        }
    }

    /// 转为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            ProxyType::Http => "http",
            ProxyType::Socks5 => "socks5",
        }
    }
}

/// 代理配置结构
/// 
/// 支持多种代理配置方式：
/// - 简单 URL 形式：proxy = "http://proxy.example.com:8080"
/// - 指定类型形式：proxy(type = http, url = "proxy.example.com:8080")
/// - 完整配置形式：proxy(type = socks5, url = "...", username = "...", password = "...")
/// - 禁用代理：proxy = false
#[derive(Clone)]
pub enum ProxyConfig {
    /// 简单的代理 URL（从 URL 自动推断类型）
    Simple(LitStr),
    /// 完整的代理配置
    Full {
        proxy_type: Option<ProxyType>,
        url: LitStr,
        username: Option<LitStr>,
        password: Option<LitStr>,
        no_proxy: Option<LitStr>,
    },
    /// 明确禁用代理
    Disabled(LitBool),
}

impl ProxyConfig {
    /// 获取代理 URL
    pub fn url(&self) -> Option<&LitStr> {
        match self {
            ProxyConfig::Simple(url) => Some(url),
            ProxyConfig::Full { url, .. } => Some(url),
            ProxyConfig::Disabled(_) => None,
        }
    }

    /// 获取代理类型
    pub fn proxy_type(&self) -> Option<&ProxyType> {
        match self {
            ProxyConfig::Full { proxy_type, .. } => proxy_type.as_ref(),
            _ => None,
        }
    }

    /// 获取用户名
    pub fn username(&self) -> Option<&LitStr> {
        match self {
            ProxyConfig::Full { username, .. } => username.as_ref(),
            _ => None,
        }
    }

    /// 获取密码
    pub fn password(&self) -> Option<&LitStr> {
        match self {
            ProxyConfig::Full { password, .. } => password.as_ref(),
            _ => None,
        }
    }

    /// 获取不使用代理的域名列表
    pub fn no_proxy(&self) -> Option<&LitStr> {
        match self {
            ProxyConfig::Full { no_proxy, .. } => no_proxy.as_ref(),
            _ => None,
        }
    }

    /// 是否明确禁用代理
    pub fn is_disabled(&self) -> bool {
        matches!(self, ProxyConfig::Disabled(_))
    }

    /// 推断代理类型（从 URL 或显式配置）
    pub fn infer_proxy_type(&self) -> Option<ProxyType> {
        match self {
            ProxyConfig::Simple(url) => {
                let url_value = url.value();
                if url_value.starts_with("http://") || url_value.starts_with("https://") {
                    Some(ProxyType::Http)
                } else if url_value.starts_with("socks5://") {
                    Some(ProxyType::Socks5)
                } else {
                    None
                }
            }
            ProxyConfig::Full { proxy_type, url, .. } => {
                if let Some(ptype) = proxy_type {
                    Some(ptype.clone())
                } else {
                    // 如果没有显式指定类型，从 URL 推断
                    let url_value = url.value();
                    if url_value.starts_with("http://") || url_value.starts_with("https://") {
                        Some(ProxyType::Http)
                    } else if url_value.starts_with("socks5://") {
                        Some(ProxyType::Socks5)
                    } else {
                        // 默认为 HTTP
                        Some(ProxyType::Http)
                    }
                }
            }
            ProxyConfig::Disabled(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::LitStr;
    use proc_macro2::Span;

    #[test]
    fn test_simple_proxy_config() {
        let url = LitStr::new("http://proxy.example.com:8080", Span::call_site());
        let config = ProxyConfig::Simple(url);
        
        assert!(config.url().is_some());
        assert_eq!(config.url().unwrap().value(), "http://proxy.example.com:8080");
        assert!(config.username().is_none());
        assert!(config.password().is_none());
        assert!(!config.is_disabled());
    }

    #[test]
    fn test_full_proxy_config() {
        let url = LitStr::new("http://proxy.example.com:8080", Span::call_site());
        let username = Some(LitStr::new("user", Span::call_site()));
        let password = Some(LitStr::new("pass", Span::call_site()));
        
        let config = ProxyConfig::Full {
            proxy_type: Some(ProxyType::Http),
            url,
            username,
            password,
            no_proxy: None,
        };
        
        assert!(config.url().is_some());
        assert_eq!(config.url().unwrap().value(), "http://proxy.example.com:8080");
        assert!(config.username().is_some());
        assert_eq!(config.username().unwrap().value(), "user");
        assert!(config.password().is_some());
        assert_eq!(config.password().unwrap().value(), "pass");
        assert!(!config.is_disabled());
    }

    #[test]
    fn test_disabled_proxy_config() {
        let disabled = LitBool::new(false, Span::call_site());
        let config = ProxyConfig::Disabled(disabled);
        
        assert!(config.url().is_none());
        assert!(config.username().is_none());
        assert!(config.password().is_none());
        assert!(config.is_disabled());
    }
}