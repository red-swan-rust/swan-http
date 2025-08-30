use quote::quote;
use syn::{FnArg, PatType};
use std::collections::HashMap;

/// 动态参数处理器
/// 
/// 处理URL和header中的动态参数占位符替换
pub struct DynamicParamsProcessor;

impl DynamicParamsProcessor {
    /// 生成带动态参数替换的URL代码
    /// 
    /// # 参数
    /// 
    /// * `url_template` - URL模板字符串，包含 {param} 占位符
    /// * `fn_inputs` - 函数参数列表
    /// 
    /// # 返回值
    /// 
    /// 生成的URL构建代码
    pub fn generate_dynamic_url_code(
        url_template: &str,
        fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    ) -> proc_macro2::TokenStream {
        let param_map = Self::extract_parameters(fn_inputs);
        
        if !Self::has_placeholders(url_template) {
            // 没有占位符，直接使用原始URL
            return quote! { 
                let full_url = format!("{}{}", self.base_url, #url_template);
            };
        }

        // 解析URL模板中的占位符
        let placeholders = Self::extract_placeholders(url_template);
        let mut format_str = url_template.to_string();
        let mut format_args = Vec::new();

        for placeholder in placeholders {
            if let Some(param_ident) = Self::resolve_placeholder(&placeholder, &param_map) {
                // 替换占位符为 Rust 格式化占位符
                format_str = format_str.replace(&format!("{{{}}}", placeholder), "{}");
                format_args.push(param_ident);
            } else {
                // 如果找不到对应参数，编译时报错
                return quote! {
                    compile_error!(concat!("Parameter '", #placeholder, "' not found in function parameters"));
                };
            }
        }

        if format_args.is_empty() {
            quote! { 
                let full_url = format!("{}{}", self.base_url, #format_str);
            }
        } else {
            quote! { 
                let full_url = format!("{}{}", self.base_url, format!(#format_str, #(#format_args),*));
            }
        }
    }

    /// 生成带动态参数替换的header代码
    /// 
    /// # 参数
    /// 
    /// * `header_template` - header模板字符串，包含 {param} 占位符
    /// * `fn_inputs` - 函数参数列表
    /// 
    /// # 返回值
    /// 
    /// 生成的header设置代码
    pub fn generate_dynamic_header_code(
        header_template: &str,
        fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    ) -> proc_macro2::TokenStream {
        let param_map = Self::extract_parameters(fn_inputs);
        
        // 解析header格式：Key: Value
        let parts: Vec<&str> = header_template.splitn(2, ": ").collect();
        if parts.len() != 2 {
            return quote! {
                compile_error!("header must be in 'Key: Value' format with a colon and space separator");
            };
        }

        let header_key = parts[0];
        let header_value_template = parts[1];

        if !Self::has_placeholders(header_value_template) {
            // 没有占位符，直接使用原始值
            return quote! {
                .header(#header_key, #header_value_template)
            };
        }

        // 解析header值模板中的占位符
        let placeholders = Self::extract_placeholders(header_value_template);
        let mut format_str = header_value_template.to_string();
        let mut format_args = Vec::new();

        for placeholder in placeholders {
            if let Some(param_ident) = Self::resolve_placeholder(&placeholder, &param_map) {
                format_str = format_str.replace(&format!("{{{}}}", placeholder), "{}");
                format_args.push(param_ident);
            } else {
                return quote! {
                    compile_error!(concat!("Parameter '", #placeholder, "' not found in function parameters"));
                };
            }
        }

        if format_args.is_empty() {
            quote! {
                .header(#header_key, #format_str)
            }
        } else {
            quote! {
                .header(#header_key, format!(#format_str, #(#format_args),*))
            }
        }
    }

    /// 检查字符串是否包含占位符
    fn has_placeholders(text: &str) -> bool {
        text.contains('{') && text.contains('}')
    }

    /// 提取字符串中的所有占位符
    fn extract_placeholders(text: &str) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = text.chars().peekable();
        
        while let Some(ch) = chars.next() {
            if ch == '{' {
                let mut placeholder = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch == '}' {
                        chars.next(); // 消费 '}'
                        break;
                    }
                    placeholder.push(chars.next().unwrap());
                }
                if !placeholder.is_empty() {
                    placeholders.push(placeholder);
                }
            }
        }
        
        placeholders
    }

    /// 从函数参数中提取参数映射
    fn extract_parameters(
        fn_inputs: &syn::punctuated::Punctuated<FnArg, syn::Token![,]>,
    ) -> HashMap<String, syn::Ident> {
        let mut param_map = HashMap::new();
        
        // 跳过self参数，从第二个参数开始
        for (index, input) in fn_inputs.iter().skip(1).enumerate() {
            if let FnArg::Typed(PatType { pat, .. }) = input {
                if let syn::Pat::Ident(pat_ident) = pat.as_ref() {
                    let param_name = pat_ident.ident.to_string();
                    
                    // 支持两种引用方式：
                    // 1. 按名称：{param_name}
                    param_map.insert(param_name.clone(), pat_ident.ident.clone());
                    
                    // 2. 按位置：{param0}, {param1}, etc.
                    param_map.insert(format!("param{}", index), pat_ident.ident.clone());
                }
            }
        }
        
        param_map
    }

    /// 解析占位符到对应的参数标识符
    fn resolve_placeholder(
        placeholder: &str,
        param_map: &HashMap<String, syn::Ident>,
    ) -> Option<syn::Ident> {
        param_map.get(placeholder).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_placeholders() {
        let text = "/users/{user_id}/posts/{post_id}";
        let placeholders = DynamicParamsProcessor::extract_placeholders(text);
        assert_eq!(placeholders, vec!["user_id", "post_id"]);
    }

    #[test]
    fn test_extract_placeholders_no_placeholders() {
        let text = "/users/123/posts/456";
        let placeholders = DynamicParamsProcessor::extract_placeholders(text);
        assert!(placeholders.is_empty());
    }

    #[test]
    fn test_has_placeholders() {
        assert!(DynamicParamsProcessor::has_placeholders("/users/{id}"));
        assert!(!DynamicParamsProcessor::has_placeholders("/users/123"));
    }

    #[test]
    fn test_extract_parameters() {
        let inputs: syn::punctuated::Punctuated<syn::FnArg, syn::Token![,]> = parse_quote! {
            &self, user_id: u32, post_id: u32
        };
        
        let param_map = DynamicParamsProcessor::extract_parameters(&inputs);
        
        // 检查按名称映射
        assert!(param_map.contains_key("user_id"));
        assert!(param_map.contains_key("post_id"));
        
        // 检查按位置映射
        assert!(param_map.contains_key("param0")); // user_id
        assert!(param_map.contains_key("param1")); // post_id
    }
}