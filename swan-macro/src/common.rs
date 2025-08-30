use crate::generator::generate_http_method;
use proc_macro::TokenStream;
use swan_common::{HttpMethod, parse_handler_args};
use syn::{ItemFn, parse_macro_input};

/// 通用 HTTP 方法处理函数
/// 
/// 这是所有 HTTP 方法宏（GET、POST、PUT、DELETE）的共同入口点。
/// 它解析宏参数和函数定义，然后调用代码生成器。
/// 
/// # 参数
/// 
/// * `args` - 宏属性参数
/// * `item` - 被标注的函数
/// * `http_method` - HTTP 方法类型
/// 
/// # 返回值
/// 
/// 生成的 TokenStream
pub fn common_http_method(
    args: TokenStream,
    item: TokenStream,
    http_method: HttpMethod,
) -> TokenStream {
    let item = parse_macro_input!(item as ItemFn);

    let mut args = parse_macro_input!(args with parse_handler_args);
    args.method = http_method;

    generate_http_method(&item.sig, &args)
}
