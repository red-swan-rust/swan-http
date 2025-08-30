use quote::quote;
use swan_common::{RetryConfig, RetryPolicy};

/// 重试机制处理器
/// 
/// 实现方法级别的渐进指数重试功能
pub struct RetryProcessor;

impl RetryProcessor {
    /// 生成重试执行代码
    /// 
    /// 根据重试配置生成完整的重试逻辑，包括指数退避、幂等性检查和条件判断
    pub fn generate_retry_execution_code(
        retry_config: &Option<RetryConfig>,
        method: &swan_common::HttpMethod,
    ) -> proc_macro2::TokenStream {
        match retry_config {
            Some(config) => {
                let policy = &config.policy;
                Self::generate_retry_with_policy(policy, method)
            }
            None => {
                // 无重试配置，直接执行
                quote! {
                    let response = self.client.execute(request).await
                        .map_err(|e| anyhow::anyhow!("Request execution failed: {}", e))?;
                }
            }
        }
    }

    /// 生成带重试策略的执行代码
    fn generate_retry_with_policy(
        policy: &RetryPolicy,
        method: &swan_common::HttpMethod,
    ) -> proc_macro2::TokenStream {
        let max_attempts = policy.max_attempts;
        let base_delay_ms = policy.base_delay_ms;
        let max_delay_ms = policy.max_delay_ms;
        let exponential_base = policy.exponential_base;
        let jitter_ratio = policy.jitter_ratio;
        let idempotent_only = policy.idempotent_only;

        // 幂等性检查
        let idempotent_check = if idempotent_only {
            Self::generate_idempotent_check(method)
        } else {
            quote! { true }
        };

        quote! {
            // 重试策略配置
            const MAX_ATTEMPTS: u32 = #max_attempts;
            const BASE_DELAY_MS: u64 = #base_delay_ms;
            const MAX_DELAY_MS: u64 = #max_delay_ms;
            const EXPONENTIAL_BASE: f64 = #exponential_base;
            const JITTER_RATIO: f64 = #jitter_ratio;
            const IS_IDEMPOTENT: bool = #idempotent_check;

            let mut last_error = None;
            let mut response = None;
            
            for attempt in 0..MAX_ATTEMPTS {
                // 克隆请求以支持重试
                let request_clone = match request.try_clone() {
                    Some(req) => req,
                    None => {
                        return Err(anyhow::anyhow!(
                            "Request body cannot be cloned for retry. Use idempotent methods or ensure request body is cloneable."
                        ));
                    }
                };

                match self.client.execute(request_clone).await {
                    Ok(resp) => {
                        let status = resp.status().as_u16();
                        
                        // 检查是否需要重试
                        if should_retry_response(status) && attempt < MAX_ATTEMPTS - 1 && IS_IDEMPOTENT {
                            log::warn!("Request failed with status {}, retrying attempt {}/{}", 
                                      status, attempt + 2, MAX_ATTEMPTS);
                            
                            // 计算延迟时间
                            let delay_ms = calculate_retry_delay(
                                attempt + 1, 
                                BASE_DELAY_MS, 
                                MAX_DELAY_MS, 
                                EXPONENTIAL_BASE, 
                                JITTER_RATIO
                            );
                            
                            // 异步延迟
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        
                        // 成功或不需要重试的响应
                        response = Some(resp);
                        break;
                    }
                    Err(e) => {
                        last_error = Some(e);
                        
                        // 网络错误重试判断
                        if attempt < MAX_ATTEMPTS - 1 && IS_IDEMPOTENT {
                            log::warn!("Network error on attempt {}/{}, retrying: {}", 
                                      attempt + 1, MAX_ATTEMPTS, last_error.as_ref().unwrap());
                            
                            let delay_ms = calculate_retry_delay(
                                attempt + 1, 
                                BASE_DELAY_MS, 
                                MAX_DELAY_MS, 
                                EXPONENTIAL_BASE, 
                                JITTER_RATIO
                            );
                            
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                        
                        // 最终失败
                        return Err(anyhow::anyhow!("Request failed after {} attempts: {}", 
                                                 attempt + 1, last_error.unwrap()));
                    }
                }
            }
            
            let response = response.ok_or_else(|| 
                anyhow::anyhow!("Retry loop completed without successful response"))?;
        }
    }

    /// 生成幂等性检查代码
    fn generate_idempotent_check(method: &swan_common::HttpMethod) -> proc_macro2::TokenStream {
        use swan_common::HttpMethod;
        match method {
            HttpMethod::Get | HttpMethod::Put | HttpMethod::Delete => quote! { true },
            HttpMethod::Post => quote! { false },
        }
    }

    /// 生成重试条件判断代码
    pub fn generate_retry_condition_code() -> proc_macro2::TokenStream {
        quote! {
            #[inline(always)]
            fn should_retry_response(status: u16) -> bool {
                match status {
                    // 5xx 服务器错误
                    500..=599 => true,
                    // 429 限流
                    429 => true,
                    // 408 请求超时
                    408 => true,
                    // 其他状态不重试
                    _ => false,
                }
            }
        }
    }

    /// 生成延迟计算代码
    pub fn generate_delay_calculation_code() -> proc_macro2::TokenStream {
        quote! {
            #[inline(always)]
            fn calculate_retry_delay(
                attempt: u32,
                base_delay_ms: u64,
                max_delay_ms: u64,
                exponential_base: f64,
                jitter_ratio: f64,
            ) -> u64 {
                if attempt == 0 {
                    return 0;
                }

                // 指数退避: base_delay * exponential_base^(attempt-1)
                let exponential_delay = base_delay_ms as f64 
                    * exponential_base.powi((attempt - 1) as i32);
                
                // 应用最大延迟限制
                let capped_delay = exponential_delay.min(max_delay_ms as f64);
                
                // 添加随机抖动 (避免雷群效应)
                let jitter = if jitter_ratio > 0.0 {
                    let max_jitter = capped_delay * jitter_ratio;
                    fastrand::f64() * max_jitter
                } else {
                    0.0
                };

                (capped_delay + jitter) as u64
            }
        }
    }

    /// 生成重试监控代码
    pub fn generate_retry_monitoring_code() -> proc_macro2::TokenStream {
        quote! {
            #[cfg(debug_assertions)]
            #[inline(always)]
            fn log_retry_attempt(attempt: u32, max_attempts: u32, delay_ms: u64, reason: &str) {
                log::debug!("Retry attempt {}/{} after {}ms delay. Reason: {}", 
                           attempt, max_attempts, delay_ms, reason);
            }
            
            #[cfg(not(debug_assertions))]
            #[inline(always)]
            fn log_retry_attempt(_attempt: u32, _max_attempts: u32, _delay_ms: u64, _reason: &str) {
                // 生产环境：无日志开销
            }
        }
    }

    /// 生成完整的重试代码块
    pub fn generate_complete_retry_block(
        retry_config: &Option<RetryConfig>,
        method: &swan_common::HttpMethod,
    ) -> proc_macro2::TokenStream {
        let retry_execution = Self::generate_retry_execution_code(retry_config, method);
        let retry_condition = Self::generate_retry_condition_code();
        let delay_calculation = Self::generate_delay_calculation_code();
        let retry_monitoring = Self::generate_retry_monitoring_code();

        quote! {
            // 重试相关工具函数
            #retry_condition
            #delay_calculation
            #retry_monitoring

            // 执行请求（包含重试逻辑）
            #retry_execution
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_generate_retry_execution_no_config() {
        let result = RetryProcessor::generate_retry_execution_code(&None, &swan_common::HttpMethod::Get);
        let result_str = result.to_string();
        assert!(result_str.contains("client.execute(request)"));
        assert!(!result_str.contains("MAX_ATTEMPTS"));
    }

    #[test]
    fn test_generate_idempotent_check_get() {
        let result = RetryProcessor::generate_idempotent_check(&swan_common::HttpMethod::Get);
        let result_str = result.to_string();
        assert_eq!(result_str.trim(), "true");
    }

    #[test]
    fn test_generate_idempotent_check_post() {
        let result = RetryProcessor::generate_idempotent_check(&swan_common::HttpMethod::Post);
        let result_str = result.to_string();
        assert_eq!(result_str.trim(), "false");
    }

    #[test]
    fn test_generate_retry_condition_code() {
        let result = RetryProcessor::generate_retry_condition_code();
        let result_str = result.to_string();
        assert!(result_str.contains("should_retry_response"));
        assert!(result_str.contains("500..=599"));
        assert!(result_str.contains("429"));
    }

    #[test]
    fn test_generate_delay_calculation_code() {
        let result = RetryProcessor::generate_delay_calculation_code();
        let result_str = result.to_string();
        assert!(result_str.contains("calculate_retry_delay"));
        assert!(result_str.contains("exponential_base"));
        assert!(result_str.contains("jitter_ratio"));
    }
}