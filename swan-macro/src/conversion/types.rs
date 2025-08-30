use quote::quote;
use syn::{GenericArgument, PathArguments, Type};

/// 生成类型转换代码
/// 
/// 根据返回类型生成相应的反序列化代码。支持以下类型：
/// - `String`: 直接从字节转换为UTF-8字符串
/// - `Vec<u8>`: 直接返回字节向量
/// - 其他类型: 使用 serde_json 进行JSON反序列化
/// 
/// # 参数
/// 
/// * `ok_type` - 成功返回的类型
/// 
/// # 返回值
/// 
/// 生成的类型转换代码
pub fn generate_type_conversion(ok_type: &GenericArgument) -> proc_macro2::TokenStream {
    let default_conversion = quote! {
        serde_json::from_slice::<#ok_type>(&bytes)?
    };

    match ok_type {
        GenericArgument::Type(Type::Path(type_path)) => {
            let last_segment = match type_path.path.segments.last() {
                Some(segment) => segment,
                None => return default_conversion,
            };

            match last_segment.ident.to_string().as_str() {
                "String" => generate_string_conversion(),
                "Vec" => generate_vec_conversion(&last_segment.arguments, &default_conversion),
                _ => default_conversion,
            }
        }
        _ => default_conversion,
    }
}

/// 生成字符串类型的转换代码
fn generate_string_conversion() -> proc_macro2::TokenStream {
    quote! {
        String::from_utf8_lossy(&bytes).to_string()
    }
}

/// 生成向量类型的转换代码
fn generate_vec_conversion(
    arguments: &PathArguments,
    default_conversion: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    match arguments {
        PathArguments::AngleBracketed(args) => {
            match args.args.first() {
                Some(GenericArgument::Type(Type::Path(inner_type))) => {
                    if let Some(last_segment) = inner_type.path.segments.last() {
                        if last_segment.ident == "u8" {
                            return quote! {
                                bytes.to_vec()
                            };
                        }
                    }
                    default_conversion.clone()
                }
                _ => default_conversion.clone(),
            }
        }
        _ => default_conversion.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, GenericArgument};

    #[test]
    fn test_string_type_conversion() {
        let ok_type: GenericArgument = parse_quote! { String };
        let result = generate_type_conversion(&ok_type);
        let expected = quote! {
            String::from_utf8_lossy(&bytes).to_string()
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_vec_u8_type_conversion() {
        let ok_type: GenericArgument = parse_quote! { Vec<u8> };
        let result = generate_type_conversion(&ok_type);
        let expected = quote! {
            bytes.to_vec()
        };
        assert_eq!(result.to_string(), expected.to_string());
    }

    #[test]
    fn test_custom_type_conversion() {
        let ok_type: GenericArgument = parse_quote! { MyCustomType };
        let result = generate_type_conversion(&ok_type);
        let expected = quote! {
            serde_json::from_slice::<MyCustomType>(&bytes)?
        };
        assert_eq!(result.to_string(), expected.to_string());
    }
}