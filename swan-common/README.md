# Swan Common

[![Crates.io](https://img.shields.io/crates/v/swan-common.svg)](https://crates.io/crates/swan-common)
[![Documentation](https://docs.rs/swan-common/badge.svg)](https://docs.rs/swan-common)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Swan Common 是 Swan HTTP 库的核心组件，提供共享的类型定义、拦截器接口和重试机制等基础功能。

## 🌟 核心功能

- **HTTP 类型定义**: 统一的 HTTP 方法、内容类型等类型定义
- **拦截器接口**: 高性能的零拷贝拦截器 trait 定义
- **重试机制**: 完整的指数退避重试策略实现
- **参数解析**: 宏参数解析和验证逻辑
- **状态管理**: 应用状态注入的类型支持

## 📦 安装

将以下内容添加到你的 `Cargo.toml`:

```toml
[dependencies]
swan-common = "0.1.0"
async-trait = "0.1"
anyhow = "1.0"
```

## 🔧 主要组件

### HTTP 类型

```rust
use swan_common::{HttpMethod, ContentType};

let method = HttpMethod::Get;
let content_type = ContentType::Json;
```

### 拦截器接口

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
        // 零拷贝：仅在需要时修改请求体
        Ok((request, Cow::Borrowed(request_body)))
    }

    async fn after_response(
        &self,
        response: reqwest::Response,
        context: Option<&(dyn Any + Send + Sync)>,
    ) -> anyhow::Result<reqwest::Response> {
        println!("响应状态: {}", response.status());
        Ok(response)
    }
}
```

### 重试策略

```rust
use swan_common::{RetryPolicy, RetryConfig};
use syn::LitStr;

// 创建指数重试策略
let policy = RetryPolicy::exponential(3, 100); // 3次重试，基础延迟100ms

// 从字符串解析重试配置
let config_str: LitStr = syn::parse_quote!("exponential(5, 200ms)");
let retry_config = RetryConfig::parse(&config_str)?;
```

## 🔄 重试机制特性

- **指数退避算法**: 智能的延迟增长，避免服务器过载
- **随机抖动**: 防止雷群效应，分散重试时间  
- **幂等性保护**: 自动检测安全的重试条件
- **灵活配置**: 支持简化和详细配置语法

### 支持的重试配置格式

```rust
// 简化格式
"exponential(3, 100ms)"           // 3次重试，基础延迟100ms
"fixed(max_attempts=4, delay=1s)" // 4次重试，固定延迟1秒

// 详细格式
"exponential(
    max_attempts=5,
    base_delay=200ms,
    max_delay=30s,
    exponential_base=2.0,
    jitter_ratio=0.1,
    idempotent_only=true
)"
```

## ⚡ 性能特性

- **零拷贝拦截器**: 使用 `Cow<[u8]>` 避免不必要的内存拷贝
- **编译时优化**: 重试策略在编译时确定，零运行时开销
- **轻量级结构**: `RetryPolicy` 内存占用 ≤ 64 bytes

## 🧪 测试

运行测试：

```bash
cargo test --lib
```

## 📖 文档

详细的 API 文档：

```bash
cargo doc --open
```

## 🤝 与 Swan Macro 配合使用

Swan Common 通常与 [Swan Macro](https://crates.io/crates/swan-macro) 配合使用：

```toml
[dependencies]
swan-common = "0.1.0"
swan-macro = "0.1.0"
```

## 📄 许可证

本项目采用 MIT 许可证。详情请查看 [LICENSE](../LICENSE) 文件。