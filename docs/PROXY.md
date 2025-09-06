# 代理支持

Swan HTTP 客户端提供全面的代理支持，包括 HTTP、HTTPS 和 SOCKS5 代理。

## 支持的代理类型

- **HTTP 代理**: `http://proxy.example.com:8080`
- **HTTPS 代理**: `https://proxy.example.com:8080`
- **SOCKS5 代理**: `socks5://proxy.example.com:1080`

## 配置语法

### 1. 客户端级别代理配置

#### 简单 URL 形式（自动类型推断）
```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy = "http://proxy.example.com:8080"
)]
struct HttpProxyClient;

#[http_client(
    base_url = "https://api.example.com",
    proxy = "socks5://proxy.example.com:1080"
)]
struct Socks5ProxyClient;
```

#### 完整配置形式（带认证）
```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy(
        url = "proxy.example.com:8080",
        username = "proxyuser",
        password = "proxypass"
    )
)]
struct AuthProxyClient;
```

#### 禁用代理
```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy = false
)]
struct NoProxyClient;
```

### 2. 方法级别代理覆盖

方法级别的代理配置可以覆盖客户端级别的设置：

```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy = "http://default-proxy.example.com:8080"
)]
struct MixedClient;

impl MixedClient {
    // 使用客户端默认代理
    #[get(url = "/default")]
    async fn with_default_proxy(&self) -> anyhow::Result<Data> {}

    // 方法级覆盖：使用 SOCKS5 代理
    #[get(url = "/secure", proxy = "socks5://secure-proxy.example.com:1080")]
    async fn with_socks_proxy(&self) -> anyhow::Result<Data> {}

    // 方法级覆盖：禁用代理
    #[get(url = "/local", proxy = false)]
    async fn without_proxy(&self) -> anyhow::Result<Data> {}
}
```

## 配置优先级

代理配置的优先级顺序（从高到低）：

1. **方法级别**: `#[get(url = "...", proxy = "...")]`
2. **客户端级别**: `#[http_client(proxy = "...")]`
3. **环境变量**: `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`

## 认证支持

对于需要认证的代理服务器，可以在完整配置形式中提供用户名和密码：

```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy(
        url = "proxy.example.com:8080",
        username = "myusername",
        password = "mypassword"
    )
)]
struct AuthenticatedProxyClient;
```

## 注意事项

1. **no_proxy 功能**: 目前 `no_proxy` 配置主要通过环境变量支持，Swan HTTP 会在使用时显示警告提示。

2. **性能优化**: 
   - 客户端级别代理：代理客户端在结构体创建时初始化
   - 方法级别代理：使用 `OnceLock` 缓存，确保高性能

3. **错误处理**: 无效的代理 URL 会在编译时或运行时产生清晰的错误信息。

## 示例代码

完整的使用示例请参考 `examples/proxy_usage.rs` 文件，其中包含了各种代理配置场景的演示。

## 环境变量支持

Swan HTTP 遵循标准的代理环境变量约定：

- `HTTP_PROXY`: HTTP 请求的代理
- `HTTPS_PROXY`: HTTPS 请求的代理  
- `ALL_PROXY`: 所有请求的代理
- `NO_PROXY`: 不使用代理的域名列表

这些环境变量会作为备用配置，当没有显式指定代理时使用。