use quote::quote;
use syn::LitStr;

/// 字符串池化和缓存优化器
/// 
/// 缓存频繁使用的字符串和URL模板，减少内存分配和字符串操作开销
pub struct StringPoolOptimizer;

impl StringPoolOptimizer {
    /// 生成字符串池化代码
    /// 
    /// 创建编译时字符串常量池，避免运行时字符串分配
    pub fn generate_string_pool_code(
        base_url: &str,
        static_urls: Vec<&str>,
        header_keys: Vec<&str>,
    ) -> proc_macro2::TokenStream {
        let base_url_const = format!("BASE_URL_{}", Self::hash_string(base_url));
        let url_consts: Vec<_> = static_urls.iter().enumerate().map(|(i, url)| {
            let const_name = format!("STATIC_URL_{}", i);
            (const_name, *url)
        }).collect();
        
        let header_consts: Vec<_> = header_keys.iter().enumerate().map(|(i, key)| {
            let const_name = format!("HEADER_KEY_{}", i);
            (const_name, *key)
        }).collect();

        let base_url_decl = quote! {
            const #base_url_const: &str = #base_url;
        };

        let url_decls = url_consts.iter().map(|(name, url)| {
            let name_ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! {
                const #name_ident: &str = #url;
            }
        });

        let header_decls = header_consts.iter().map(|(name, key)| {
            let name_ident = syn::Ident::new(name, proc_macro2::Span::call_site());
            quote! {
                const #name_ident: &str = #key;
            }
        });

        quote! {
            // 编译时字符串常量池
            #base_url_decl
            #(#url_decls)*
            #(#header_decls)*
            
            /// 字符串池化容器
            pub struct StringPool {
                // 运行时动态字符串缓存
                url_cache: std::sync::RwLock<HashMap<String, String>>,
                header_cache: std::sync::RwLock<HashMap<String, String>>,
            }
            
            impl StringPool {
                pub fn new() -> Self {
                    Self {
                        url_cache: std::sync::RwLock::new(HashMap::with_capacity(16)),
                        header_cache: std::sync::RwLock::new(HashMap::with_capacity(8)),
                    }
                }
                
                /// 获取或缓存URL字符串
                #[inline(always)]
                pub fn get_or_cache_url(&self, template: &str, params: &[&str]) -> String {
                    // 尝试从缓存读取
                    {
                        let cache = self.url_cache.read().unwrap();
                        if let Some(cached) = cache.get(template) {
                            return cached.clone();
                        }
                    }
                    
                    // 缓存未命中，生成并缓存
                    let mut url = template.to_string();
                    for (i, param) in params.iter().enumerate() {
                        let placeholder = format!("{{param{}}}", i);
                        url = url.replace(&placeholder, param);
                    }
                    
                    // 写入缓存
                    let mut cache = self.url_cache.write().unwrap();
                    cache.insert(template.to_string(), url.clone());
                    url
                }
                
                /// 获取或缓存header值
                #[inline(always)]
                pub fn get_or_cache_header(&self, key: &str, value_template: &str, params: &[&str]) -> (String, String) {
                    let cache_key = format!("{}:{}", key, value_template);
                    
                    // 尝试从缓存读取
                    {
                        let cache = self.header_cache.read().unwrap();
                        if let Some(cached_value) = cache.get(&cache_key) {
                            return (key.to_string(), cached_value.clone());
                        }
                    }
                    
                    // 缓存未命中，生成并缓存
                    let mut value = value_template.to_string();
                    for (i, param) in params.iter().enumerate() {
                        let placeholder = format!("{{param{}}}", i);
                        value = value.replace(&placeholder, param);
                    }
                    
                    // 写入缓存
                    let mut cache = self.header_cache.write().unwrap();
                    cache.insert(cache_key, value.clone());
                    (key.to_string(), value)
                }
            }
            
            impl Default for StringPool {
                fn default() -> Self {
                    Self::new()
                }
            }
        }
    }

    /// 生成URL模板编译器代码
    /// 
    /// 在编译时预处理URL模板，生成高效的运行时替换代码
    pub fn generate_url_template_compiler(url_template: &str) -> proc_macro2::TokenStream {
        let placeholders = Self::extract_placeholders(url_template);
        let optimized_template = Self::optimize_url_template(url_template, &placeholders);
        
        let param_idents: Vec<_> = placeholders.iter().map(|&i| {
            syn::Ident::new(&format!("param{}", i), proc_macro2::Span::call_site())
        }).collect();
        
        quote! {
            #[inline(always)]
            fn compile_url_template(#(#param_idents: &str),*) -> String {
                // 编译时优化的URL构建
                #optimized_template
            }
        }
    }

    /// 生成header模板编译器代码
    pub fn generate_header_template_compiler(
        headers: &syn::punctuated::Punctuated<LitStr, syn::Token![,]>
    ) -> proc_macro2::TokenStream {
        let header_compilers: Vec<_> = headers.iter().map(|header| {
            let header_str = header.value();
            Self::generate_single_header_compiler(&header_str)
        }).collect();

        quote! {
            #(#header_compilers)*
        }
    }

    /// 提取占位符
    fn extract_placeholders(template: &str) -> Vec<usize> {
        let mut placeholders = Vec::new();
        let mut chars = template.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut placeholder = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next(); // 消耗 '}'
                        break;
                    }
                    placeholder.push(chars.next().unwrap());
                }
                
                // 解析占位符索引
                if placeholder.starts_with("param") {
                    if let Ok(index) = placeholder[5..].parse::<usize>() {
                        if !placeholders.contains(&index) {
                            placeholders.push(index);
                        }
                    }
                }
            }
        }
        
        placeholders.sort();
        placeholders
    }

    /// 优化URL模板
    fn optimize_url_template(template: &str, placeholders: &[usize]) -> proc_macro2::TokenStream {
        if placeholders.is_empty() {
            // 无占位符，直接返回静态字符串
            quote! {
                #template.to_string()
            }
        } else {
            // 有占位符，生成优化的字符串拼接
            let mut parts = vec![template.to_string()];
            for &index in placeholders {
                let placeholder = format!("{{param{}}}", index);
                let _param_ident = syn::Ident::new(&format!("param{}", index), proc_macro2::Span::call_site());
                
                // 分割并重组
                for part in &mut parts {
                    if part.contains(&placeholder) {
                        *part = part.replace(&placeholder, "{}");
                    }
                }
            }
            
            // 生成format!调用
            let format_str = parts[0].clone();
            let param_idents: Vec<_> = placeholders.iter().map(|&i| {
                syn::Ident::new(&format!("param{}", i), proc_macro2::Span::call_site())
            }).collect();
            
            quote! {
                format!(#format_str, #(#param_idents),*)
            }
        }
    }

    /// 生成单个header编译器
    fn generate_single_header_compiler(header: &str) -> proc_macro2::TokenStream {
        let parts: Vec<&str> = header.splitn(2, ": ").collect();
        if parts.len() == 2 {
            let key = parts[0];
            let value_template = parts[1];
            let fn_name = syn::Ident::new(&format!("compile_header_{}", Self::sanitize_identifier(key)), proc_macro2::Span::call_site());
            
            quote! {
                #[inline(always)]
                fn #fn_name() -> (&'static str, String) {
                    (#key, #value_template.to_string())
                }
            }
        } else {
            quote! {
                compile_error!("Invalid header format");
            }
        }
    }

    /// 清理标识符，确保是有效的Rust标识符
    fn sanitize_identifier(s: &str) -> String {
        s.chars()
            .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
            .collect()
    }

    /// 简单字符串哈希（用于生成常量名）
    fn hash_string(s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_placeholders() {
        let template = "/api/users/{param0}/posts/{param1}";
        let placeholders = StringPoolOptimizer::extract_placeholders(template);
        assert_eq!(placeholders, vec![0, 1]);
    }

    #[test]
    fn test_extract_placeholders_empty() {
        let template = "/api/users/static";
        let placeholders = StringPoolOptimizer::extract_placeholders(template);
        assert!(placeholders.is_empty());
    }

    #[test]
    fn test_string_pool_generation() {
        let result = StringPoolOptimizer::generate_string_pool_code(
            "https://api.example.com",
            vec!["/users", "/posts"],
            vec!["Authorization", "Content-Type"]
        );
        let result_str = result.to_string();
        assert!(result_str.contains("StringPool"));
        assert!(result_str.contains("RwLock"));
        assert!(result_str.contains("get_or_cache_url"));
    }

    #[test]
    fn test_url_template_compiler() {
        let result = StringPoolOptimizer::generate_url_template_compiler("/users/{param0}");
        let result_str = result.to_string();
        assert!(result_str.contains("compile_url_template"));
        assert!(result_str.contains("param0"));
    }
}