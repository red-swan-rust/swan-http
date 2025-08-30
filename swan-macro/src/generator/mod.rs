pub mod client;
pub mod method;

pub use client::generate_http_client_impl;
pub use method::generate_http_method;