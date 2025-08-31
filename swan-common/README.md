# Swan Common

[![Crates.io](https://img.shields.io/crates/v/swan-common.svg)](https://crates.io/crates/swan-common)
[![Documentation](https://docs.rs/swan-common/badge.svg)](https://docs.rs/swan-common)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue.svg)](https://opensource.org/licenses/GPL-3.0)

🌏 **Languages**: [English](README.md) | [中文](README_CN.md)

Swan Common is the core component of the Swan HTTP library, providing shared type definitions, interceptor interfaces, retry mechanisms, and other foundational features.

## 🌟 Core Features

- **HTTP Type Definitions**: Unified HTTP method, content type, and other type definitions
- **Interceptor Interface**: High-performance zero-copy interceptor trait definitions
- **Retry Mechanism**: Complete exponential backoff retry strategy implementation
- **Parameter Parsing**: Macro parameter parsing and validation logic
- **State Management**: Type support for application state injection

## 📦 Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
swan-common = "0.2"
async-trait = "0.1"
anyhow = "1.0"
```

## 🔧 Main Components

### HTTP Types

```rust
use swan_common::{HttpMethod, ContentType};

let method = HttpMethod::Get;
let content_type = ContentType::Json;
```

### Interceptor Interface

```rust
use async_trait::async_trait;
use swan_common::SwanInterceptor;
use std::borrow::Cow;
use std::any::Any;

#[derive(Default)]
struct MyInterceptor;

#[async_trait]
impl SwanInterceptor for MyInterceptor {
    async fn before_request<'a>(
        &self,
        request: reqwest::RequestBuilder,
        request_body: &'a [u8],
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
        // Zero-copy: only modify request body when needed
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("Response status: {}", response.status());
        Ok(response)
    }
}
```

### Retry Strategy

```rust
use swan_common::{RetryPolicy, RetryConfig};
use syn::LitStr;

// Create exponential retry strategy
let policy = RetryPolicy::exponential(3, 100); // 3 retries, base delay 100ms

// Parse retry configuration from string
let config_str: LitStr = syn::parse_quote!("exponential(5, 200ms)");
let retry_config = RetryConfig::parse(&config_str)?;
```

## 🔄 Retry Mechanism Features

- **Exponential Backoff Algorithm**: Intelligent delay growth to avoid server overload
- **Random Jitter**: Prevent thundering herd effect by spreading retry times  
- **Idempotency Protection**: Automatically detect safe retry conditions
- **Flexible Configuration**: Support both simplified and detailed configuration syntax

### Supported Retry Configuration Formats

```rust
// Simplified format
"exponential(3, 100ms)"           // 3 retries, base delay 100ms
"fixed(max_attempts=4, delay=1s)" // 4 retries, fixed delay 1 second

// Detailed format
"exponential(
    max_attempts=5,
    base_delay=200ms,
    max_delay=30s,
    exponential_base=2.0,
    jitter_ratio=0.1,
    idempotent_only=true
)"
```

## ⚡ Performance Features

- **Zero-Copy Interceptors**: Use `Cow<[u8]>` to avoid unnecessary memory copying
- **Compile-Time Optimization**: Retry strategies determined at compile time with zero runtime overhead
- **Lightweight Structures**: `RetryPolicy` memory footprint ≤ 64 bytes

## 🧪 Testing

Run tests:

```bash
cargo test --lib
```

## 📖 Documentation

Detailed API documentation:

```bash
cargo doc --open
```

## 🤝 Use with Swan Macro

Swan Common is typically used with [Swan Macro](https://crates.io/crates/swan-macro):

```toml
[dependencies]
swan-common = "0.2"
swan-macro = "0.2"
```

## 📄 License

This project is licensed under the GPL-3.0 License. See the [LICENSE](../LICENSE) file for details.