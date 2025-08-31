# Swan HTTP Retry Mechanism

🌏 **Languages**: [English](RETRY_MECHANISM_EN.md) | [中文](RETRY_MECHANISM.md)

## Overview

Swan HTTP provides a powerful and flexible method-level retry mechanism that supports progressive exponential backoff algorithms, helping handle network instability and temporary service unavailability.

## Syntax Overview

```rust
// 🔥 Minimal configuration - Recommended for beginners
#[get(url = "/api", retry = "exponential(3, 100ms)")]
//                           ↑        ↑     ↑
//                        strategy  count  delay

// 🔧 Full configuration - Recommended for production
#[get(url = "/api", retry = "exponential(
    max_attempts=5,      // Maximum 5 retry attempts
    base_delay=200ms,    // Base delay 200 milliseconds
    max_delay=30s,       // Maximum delay 30 seconds
    jitter_ratio=0.1     // 10% random jitter
)")]

// 📌 Fixed delay - Predictable timing
#[get(url = "/api", retry = "fixed(3, 1s)")]
//                          ↑        ↑  ↑
//                       strategy count delay
```

> **💡 Tips**: 
> - Use simplified syntax to get started quickly
> - Production environments recommend full syntax for clearer parameters
> - All configurations are validated at compile time, zero runtime overhead

## Core Features

- **Method-level configuration**: Independent retry strategy configuration on each HTTP method
- **Exponential backoff algorithm**: Smart delay growth to avoid server overload
- **Random jitter**: Prevents thundering herd effect, spreads retry timing
- **Idempotency protection**: Automatically detects HTTP method idempotency to ensure safe retries
- **Smart retry conditions**: Intelligent retry decisions based on HTTP status codes
- **High performance**: Compile-time optimization, zero additional runtime overhead

## Basic Usage

### retry Attribute Syntax

The retry attribute supports two syntax formats:

#### 1. Simplified Syntax (Quick Configuration)
```rust
retry = "strategy(param1, param2)"
```

#### 2. Full Syntax (Detailed Configuration)
```rust
retry = "strategy(param_name1=value1, param_name2=value2, ...)"
```

### Simple Retry Configuration

```rust
use swan_macro::{http_client, get, post};

#[http_client(base_url = "https://api.example.com")]
struct ApiClient;

impl ApiClient {
    /// Simplified syntax: exponential retry, max 3 attempts, base delay 100ms
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    /// Full syntax: fixed delay retry
    #[get(url = "/posts", retry = "fixed(max_attempts=5, delay=500ms)")]
    async fn get_posts(&self) -> anyhow::Result<Vec<Post>> {}
    
    /// No retry configuration (default behavior)
    #[post(url = "/users")]
    async fn create_user(&self, user: User) -> anyhow::Result<User> {}
}
```

## Retry Strategies

### Exponential Backoff Retry (exponential)

Exponential backoff is the recommended retry strategy with exponentially growing delays, suitable for most scenarios.

#### Syntax Format

**Simplified syntax:**
```rust
retry = "exponential(max_attempts, base_delay)"
```

**Full syntax:**
```rust
retry = "exponential(param_name=value, param_name=value, ...)"
```

#### Usage Examples

```rust
// 📝 Simplified syntax examples
#[get(url = "/api/data", retry = "exponential(3, 100ms)")]
async fn get_data(&self) -> anyhow::Result<Data> {}

// 📝 Full syntax examples
#[get(url = "/api/data", retry = "exponential(
    max_attempts=5,
    base_delay=200ms,
    max_delay=30s,
    exponential_base=2.0,
    jitter_ratio=0.1,
    idempotent_only=true
)")]
async fn get_data_advanced(&self) -> anyhow::Result<Data> {}
```

#### Parameter Details

| Parameter | Type | Default | Description | Example Values |
|-----------|------|---------|-------------|----------------|
| `max_attempts` | integer | required | Maximum retry attempts (including initial request) | `3`, `5`, `10` |
| `base_delay` | time | required | Base delay time | `100ms`, `1s`, `500ms` |
| `max_delay` | time | `60s` | Maximum delay ceiling | `10s`, `60s`, `300s` |
| `exponential_base` | float | `2.0` | Exponential growth base | `1.5`, `2.0`, `3.0` |
| `jitter_ratio` | float | `0.1` | Random jitter ratio (0.0-1.0) | `0.0`, `0.1`, `0.5` |
| `idempotent_only` | boolean | `true` | Whether to retry only idempotent methods | `true`, `false` |

#### Time Unit Support
- `ms` : milliseconds 
- `s` : seconds

```rust
// ✅ Supported time formats
retry = "exponential(3, 100ms)"      // 100 milliseconds
retry = "exponential(3, 2s)"         // 2 seconds
retry = "exponential(max_attempts=3, base_delay=1500ms)"  // 1.5 seconds
```

**Delay Calculation Formula:**
```
delay = min(base_delay * exponential_base^(attempt-1) + jitter, max_delay)
```

### Fixed Delay Retry (fixed)

Fixed delay retry uses the same delay time for each retry, suitable for stable service environments.

#### Syntax Format

**Simplified syntax:**
```rust
retry = "fixed(max_attempts, delay)"
```

**Full syntax:**
```rust
retry = "fixed(max_attempts=count, delay=time)"
```

#### Usage Examples

```rust
// 📝 Simplified syntax examples
#[get(url = "/api/data", retry = "fixed(3, 1s)")]
async fn get_data(&self) -> anyhow::Result<Data> {}

// 📝 Full syntax examples
#[get(url = "/api/data", retry = "fixed(max_attempts=5, delay=500ms)")]
async fn get_data_detailed(&self) -> anyhow::Result<Data> {}
```

#### Parameter Details

| Parameter | Type | Default | Description | Example Values |
|-----------|------|---------|-------------|----------------|
| `max_attempts` | integer | required | Maximum retry attempts (including initial request) | `3`, `5`, `10` |
| `delay` | time | required | Fixed delay time for each retry | `100ms`, `1s`, `2s` |

## Quick Reference

### Common Configuration Templates

```rust
// 🚀 Quick retry (microservice internal calls)
retry = "exponential(3, 50ms)"

// 🌐 Standard retry (external API calls)
retry = "exponential(5, 200ms)"

// 🔄 Gentle retry (rate-limit sensitive services)
retry = "exponential(max_attempts=7, base_delay=1s, max_delay=60s, jitter_ratio=0.3)"

// ⏱️ Fixed delay (predictable scenarios)
retry = "fixed(4, 1s)"

// ⚠️ Force retry non-idempotent methods (use with caution)
retry = "exponential(max_attempts=3, base_delay=100ms, idempotent_only=false)"
```

### Syntax Comparison Table

| Configuration | exponential simplified | exponential full | fixed simplified | fixed full |
|--------------|----------------------|------------------|------------------|------------|
| **Format** | `exponential(count, delay)` | `exponential(param=value, ...)` | `fixed(count, delay)` | `fixed(param=value, ...)` |
| **Example** | `exponential(3, 100ms)` | `exponential(max_attempts=3, base_delay=100ms)` | `fixed(3, 1s)` | `fixed(max_attempts=3, delay=1s)` |
| **Pros** | Concise and clear | Good readability, complete parameters | Simple syntax | Clear parameter meaning |
| **Recommended** | Quick configuration | Production detailed configuration | Simple scenarios | Clear configuration needs |

## Retry Conditions

### Automatically Retried Status Codes

- **5xx Server Errors** (500-599): Server internal errors, usually temporary
- **429 Too Many Requests**: Rate limiting, server overload
- **408 Request Timeout**: Request timeout

### Non-retried Status Codes

- **2xx Success Responses**: Request successful
- **4xx Client Errors** (except 408, 429): Client request issues, retry meaningless

### Network Errors

All network connection errors (such as connection timeout, DNS resolution failure, etc.) will trigger retries.

## Idempotency Protection

### What is Idempotency?

Idempotent operations are those that produce the same result when executed multiple times. In HTTP:

- **Idempotent methods**: GET, PUT, DELETE
- **Non-idempotent methods**: POST

### Safe Retries

By default, only idempotent methods will automatically retry:

```rust
impl ApiClient {
    /// GET method: automatic retry ✅
    #[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
    async fn get_user(&self, id: u32) -> anyhow::Result<User> {}
    
    /// POST method: no retry by default ⚠️
    #[post(url = "/users", retry = "exponential(3, 100ms)")]  // Won't actually retry
    async fn create_user(&self, user: User) -> anyhow::Result<User> {}
    
    /// POST method: force retry ⚠️ (use with caution)
    #[post(url = "/idempotent-action", retry = "exponential(
        max_attempts=3, 
        base_delay=100ms, 
        idempotent_only=false
    )")]
    async fn safe_post_action(&self, data: Data) -> anyhow::Result<Response> {}
}
```

## Configuration Examples

### Microservice Scenarios

Quick retry, suitable for internal service calls:

```rust
#[get(url = "/internal/service", retry = "exponential(3, 50ms)")]
async fn call_internal_service(&self) -> anyhow::Result<ServiceResponse> {}
```

### External API Scenarios

Gentle retry, considering external service load:

```rust
#[get(url = "/external/api", retry = "exponential(
    max_attempts=5,
    base_delay=500ms,
    max_delay=30s,
    exponential_base=1.5,
    jitter_ratio=0.3
)")]
async fn call_external_api(&self) -> anyhow::Result<ExternalData> {}
```

### Rate-limit Sensitive Scenarios

Longer delays and gentle growth, dealing with rate limiting:

```rust
#[get(url = "/rate-limited-api", retry = "exponential(
    max_attempts=7,
    base_delay=1s,
    max_delay=60s,
    exponential_base=1.2,
    jitter_ratio=0.5
)")]
async fn call_rate_limited_api(&self) -> anyhow::Result<Data> {}
```

### Stable Service Scenarios

Fixed delay, predictable retry timing:

```rust
#[get(url = "/stable/service", retry = "fixed(max_attempts=4, delay=1s)")]
async fn call_stable_service(&self) -> anyhow::Result<Data> {}
```

## Best Practices

### 1. Choose Appropriate Retry Strategy

- **Microservice internal calls**: Use quick exponential retry `exponential(3, 50ms)`
- **External API calls**: Use gentle retry `exponential(5, 500ms)`
- **Rate-limit sensitive**: Use long delay and large jitter `exponential(7, 1s, jitter_ratio=0.5)`
- **Predictable scenarios**: Use fixed delay `fixed(3, 1s)`

### 2. Set Parameters Reasonably

#### ✅ Recommended Configurations

```rust
// 🎯 Standard scenarios - Balance performance and stability
#[get(url = "/api/users", retry = "exponential(3, 100ms)")]

// 🎯 Detailed configuration - Recommended for production
#[get(url = "/api/data", retry = "exponential(
    max_attempts=3,      // Moderate retry count (2 retries)
    base_delay=100ms,    // Reasonable base delay
    max_delay=10s,       // Prevent excessive delay
    jitter_ratio=0.1     // Moderate jitter (10% randomness)
)")]

// 🎯 Fixed delay - Predictable retry timing
#[get(url = "/stable/api", retry = "fixed(3, 500ms)")]
```

#### ❌ Not Recommended Configurations

```rust
// ❌ Excessive retries
#[get(url = "/api", retry = "exponential(50, 100ms)")]  // Too many attempts

// ❌ Improper delay settings  
#[get(url = "/api", retry = "exponential(3, 1ms)")]     // Too short delay, thundering herd
#[get(url = "/api", retry = "exponential(3, 1h)")]      // Too long delay, user waiting

// ❌ Unreasonable parameter configuration
#[get(url = "/api", retry = "exponential(
    max_attempts=3,
    base_delay=100ms,
    max_delay=50ms       // max_delay < base_delay, meaningless
)")]
```

#### 📊 Parameter Setting Guide

| Scenario | max_attempts | base_delay | max_delay | Description |
|----------|--------------|------------|-----------|-------------|
| **Internal Service** | 2-3 | 50-100ms | 5-10s | Fast failure, avoid cascading |
| **External API** | 3-5 | 200-500ms | 30-60s | Consider network latency |
| **Rate-limited Service** | 5-7 | 1-2s | 60-300s | Give service recovery time |
| **Batch Operations** | 3-5 | 500ms-1s | 30-60s | Balance throughput and latency |

### 3. Pay Attention to Idempotency

```rust
// ✅ Safe retries
#[get(url = "/users/{id}", retry = "exponential(3, 100ms)")]
#[put(url = "/users/{id}", retry = "exponential(3, 100ms)")]
#[delete(url = "/users/{id}", retry = "exponential(3, 100ms)")]

// ⚠️ Use with caution
#[post(url = "/orders", retry = "exponential(
    max_attempts=3,
    base_delay=100ms,
    idempotent_only=false  // Explicitly allow non-idempotent retry
)")]
```

### 4. Monitoring and Debugging

Enable debug logging in development environment:

```rust
// In main function
env_logger::init();

// Or more detailed configuration
env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init();
```

Log output example:
```
WARN: Request failed with status 503, retrying attempt 2/3
DEBUG: Retry attempt 2/3 after 200ms delay. Reason: Service Unavailable
```

## Error Handling

### Errors After Retry Failure

When all retries fail, the error from the last attempt is returned:

```rust
match client.get_data_with_retry().await {
    Ok(data) => println!("Success: {:?}", data),
    Err(e) => {
        // e contains the error information from the last retry
        eprintln!("Retry failed: {}", e);
    }
}
```

### Retry Errors for Non-idempotent Methods

When trying to retry non-idempotent methods with `idempotent_only=true`:

```rust
// POST methods won't actually retry by default, even with retry parameter configured
#[post(url = "/users", retry = "exponential(3, 100ms)")]
async fn create_user(&self, user: User) -> anyhow::Result<User> {}
```

## Performance Considerations

### Memory Usage

`RetryPolicy` struct is optimized with memory usage ≤ 64 bytes, suitable for high-frequency use.

### Computational Performance

Delay calculation algorithm is highly optimized:
- 1000 delay calculations < 10ms
- 100 configuration parsing < 100ms

### Concurrency Safety

Retry mechanism is completely thread-safe, supporting high-concurrency scenarios.

## Troubleshooting

### Common Issues

1. **Retry not effective**
   - Check if HTTP method is idempotent (GET/PUT/DELETE)
   - Confirm `idempotent_only` setting
   - Verify if status code is in retry range

2. **Retry time too long**
   - Reduce `max_attempts`
   - Lower `exponential_base`
   - Set reasonable `max_delay`

3. **Configuration parsing error**
   - Check if syntax format is correct
   - Confirm time units (ms/s)
   - Verify parameter name spelling

### Debugging Tips

```rust
// Enable verbose logging
RUST_LOG=debug cargo run --example retry_integration_test

// Test specific retry configuration
#[get(url = "/test", retry = "exponential(
    max_attempts=2,    // Reduce retry count for easier observation
    base_delay=1s,     // Increase delay for easier observation
    jitter_ratio=0.0   // No jitter, predictable timing
)")]
```

### Common Configuration Errors

#### ❌ Syntax Errors
```rust
// Error: missing quotes
#[get(url = "/api", retry = exponential(3, 100ms))]

// Error: wrong time unit
#[get(url = "/api", retry = "exponential(3, 100)")]      // Missing unit
#[get(url = "/api", retry = "exponential(3, 100mil)")]   // Wrong unit

// Error: parameter name typo
#[get(url = "/api", retry = "exponential(max_attempt=3, base_delay=100ms)")]  // missing 's' in attempts
```

#### ✅ Correct Syntax
```rust
// Correct: full syntax with quotes
#[get(url = "/api", retry = "exponential(3, 100ms)")]

// Correct: use correct time units
#[get(url = "/api", retry = "exponential(3, 100ms)")]    // milliseconds
#[get(url = "/api", retry = "exponential(3, 2s)")]       // seconds

// Correct: complete and accurate parameter names
#[get(url = "/api", retry = "exponential(max_attempts=3, base_delay=100ms)")]
```

#### 🔧 Compile-time Error Messages

When configuration is wrong, the compiler will provide clear error messages:

```bash
error: Invalid retry configuration: expected 'ms' or 's' for time unit
  --> src/lib.rs:10:5
   |
10 |     #[get(url = "/api", retry = "exponential(3, 100)")]
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

error: Unknown parameter 'max_attempt', did you mean 'max_attempts'?
  --> src/lib.rs:15:5
   |
15 |     #[get(url = "/api", retry = "exponential(max_attempt=3, base_delay=100ms)")]
   |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## Advanced Usage

### Custom Retry Conditions

While default retry conditions cover most scenarios, you can achieve special requirements by combining different configurations:

```rust
// Aggressive retry: more attempts, faster growth
#[get(url = "/critical-service", retry = "exponential(
    max_attempts=10,
    base_delay=10ms,
    max_delay=5s,
    exponential_base=3.0
)")]

// Conservative retry: fewer attempts, gentle growth
#[get(url = "/unstable-service", retry = "exponential(
    max_attempts=3,
    base_delay=2s,
    max_delay=30s,
    exponential_base=1.2
)")]
```

### Scenario-specific Configuration

```rust
impl ApiClient {
    // 🔥 High-frequency microservice calls
    #[get(url = "/internal/health", retry = "exponential(3, 25ms)")]
    async fn health_check(&self) -> anyhow::Result<HealthStatus> {}
    
    // 🌐 Third-party API integration
    #[get(url = "/external/weather", retry = "exponential(
        max_attempts=5,
        base_delay=1s,
        max_delay=60s,
        jitter_ratio=0.3
    )")]
    async fn get_weather(&self, city: String) -> anyhow::Result<Weather> {}
    
    // 📊 Data analytics service (may take longer to process)
    #[get(url = "/analytics/report", retry = "exponential(
        max_attempts=7,
        base_delay=2s,
        max_delay=300s,
        exponential_base=1.5
    )")]
    async fn generate_report(&self, params: ReportParams) -> anyhow::Result<Report> {}
}