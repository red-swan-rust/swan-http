# Proxy Support

Swan HTTP client provides comprehensive proxy support, including HTTP, HTTPS, and SOCKS5 proxies.

## Supported Proxy Types

- **HTTP Proxy**: `http://proxy.example.com:8080`
- **HTTPS Proxy**: `https://proxy.example.com:8080`
- **SOCKS5 Proxy**: `socks5://proxy.example.com:1080`

## Configuration Syntax

### 1. Client-Level Proxy Configuration

#### Simple URL Form (Automatic Type Inference)
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

#### Full Configuration Form (With Authentication)
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

#### Disable Proxy
```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy = false
)]
struct NoProxyClient;
```

### 2. Method-Level Proxy Override

Method-level proxy configuration can override client-level settings:

```rust
#[http_client(
    base_url = "https://api.example.com",
    proxy = "http://default-proxy.example.com:8080"
)]
struct MixedClient;

impl MixedClient {
    // Use client default proxy
    #[get(url = "/default")]
    async fn with_default_proxy(&self) -> anyhow::Result<Data> {}

    // Method-level override: Use SOCKS5 proxy
    #[get(url = "/secure", proxy = "socks5://secure-proxy.example.com:1080")]
    async fn with_socks_proxy(&self) -> anyhow::Result<Data> {}

    // Method-level override: Disable proxy
    #[get(url = "/local", proxy = false)]
    async fn without_proxy(&self) -> anyhow::Result<Data> {}
}
```

## Configuration Priority

Proxy configuration priority (from high to low):

1. **Method Level**: `#[get(url = "...", proxy = "...")]`
2. **Client Level**: `#[http_client(proxy = "...")]`
3. **Environment Variables**: `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`

## Authentication Support

For proxy servers that require authentication, you can provide username and password in the full configuration form:

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

## Notes

1. **no_proxy functionality**: Currently `no_proxy` configuration is mainly supported through environment variables, Swan HTTP will display a warning when used.

2. **Performance Optimization**: 
   - Client-level proxy: Proxy client is initialized when the struct is created
   - Method-level proxy: Uses `OnceLock` caching for high performance

3. **Error Handling**: Invalid proxy URLs will produce clear error messages at compile time or runtime.

## Example Code

For complete usage examples, please refer to the `examples/proxy_usage.rs` file, which includes demonstrations of various proxy configuration scenarios.

## Environment Variable Support

Swan HTTP follows standard proxy environment variable conventions:

- `HTTP_PROXY`: Proxy for HTTP requests
- `HTTPS_PROXY`: Proxy for HTTPS requests  
- `ALL_PROXY`: Proxy for all requests
- `NO_PROXY`: List of domains that should not use proxy

These environment variables serve as fallback configuration when no proxy is explicitly specified.