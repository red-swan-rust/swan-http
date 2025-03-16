use crate::generate::generate_http_method;
use proc_macro::TokenStream;
use swan_common::{HttpMethod, parse_handler_args};
use syn::{ItemFn, parse_macro_input};

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
