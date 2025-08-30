use quote::quote;
use syn::{GenericArgument, PathArguments, ReturnType, Type};

/// 错误处理器
/// 
/// 负责生成错误处理和类型验证相关的代码。
pub struct ErrorHandler;

impl ErrorHandler {
    /// 验证并解析返回类型
    /// 
    /// 确保函数返回类型为 `anyhow::Result<T>` 格式，并提取 Ok 和 Err 类型。
    /// 
    /// # 参数
    /// 
    /// * `output` - 函数返回类型
    /// 
    /// # 返回值
    /// 
    /// 返回 (Ok类型, Err类型) 的元组，或者语法错误
    pub fn validate_and_extract_return_types(
        output: &ReturnType,
    ) -> Result<(&GenericArgument, proc_macro2::TokenStream), syn::Error> {
        match output {
            ReturnType::Type(_, ty) => {
                let type_path = match &**ty {
                    Type::Path(type_path) => type_path,
                    _ => {
                        return Err(syn::Error::new_spanned(ty, "Return type must be anyhow::Result<T>"));
                    }
                };

                Self::validate_anyhow_result_path(type_path)?;
                let ok_type = Self::extract_ok_type(type_path, ty)?;
                let err_type = quote! { anyhow::Error };

                Ok((ok_type, err_type))
            }
            _ => Err(syn::Error::new_spanned(output, "Function must return anyhow::Result<T>")),
        }
    }

    /// 验证路径是否为 anyhow::Result
    fn validate_anyhow_result_path(type_path: &syn::TypePath) -> Result<(), syn::Error> {
        let first_segment = type_path.path.segments.first()
            .ok_or_else(|| syn::Error::new_spanned(type_path, "Return type path must not be empty"))?;

        if first_segment.ident != "anyhow" {
            return Err(syn::Error::new_spanned(type_path, "Return type must be anyhow::Result<T>"));
        }

        let last_segment = type_path.path.segments.last()
            .ok_or_else(|| syn::Error::new_spanned(type_path, "Return type path must not be empty"))?;

        if last_segment.ident != "Result" {
            return Err(syn::Error::new_spanned(type_path, "Return type must be anyhow::Result<T>"));
        }

        Ok(())
    }

    /// 提取 Ok 类型
    fn extract_ok_type<'a>(
        type_path: &'a syn::TypePath,
        ty: &'a Type,
    ) -> Result<&'a GenericArgument, syn::Error> {
        let last_segment = type_path.path.segments.last().unwrap();
        let args = match &last_segment.arguments {
            PathArguments::AngleBracketed(args) => args,
            _ => {
                return Err(syn::Error::new_spanned(
                    ty,
                    "anyhow::Result<T> must have generic arguments",
                ));
            }
        };

        if args.args.len() != 1 {
            return Err(syn::Error::new_spanned(
                ty,
                "anyhow::Result<T> must have exactly 1 type parameter",
            ));
        }

        Ok(&args.args[0])
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::{parse_quote, ReturnType};

    #[test]
    fn test_validate_correct_return_type() {
        let return_type: ReturnType = parse_quote! { -> anyhow::Result<String> };
        let result = ErrorHandler::validate_and_extract_return_types(&return_type);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_incorrect_return_type() {
        let return_type: ReturnType = parse_quote! { -> Result<String, Error> };
        let result = ErrorHandler::validate_and_extract_return_types(&return_type);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_no_return_type() {
        let return_type = ReturnType::Default;
        let result = ErrorHandler::validate_and_extract_return_types(&return_type);
        assert!(result.is_err());
    }
}