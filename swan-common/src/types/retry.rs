use std::time::Duration;
use syn::LitStr;

/// 重试策略配置
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 基础延迟时间（毫秒）
    pub base_delay_ms: u64,
    /// 最大延迟时间（毫秒）
    pub max_delay_ms: u64,
    /// 指数底数
    pub exponential_base: f64,
    /// 随机抖动比例 (0.0-1.0)
    pub jitter_ratio: f64,
    /// 仅对幂等方法重试
    pub idempotent_only: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 30000, // 30秒
            exponential_base: 2.0,
            jitter_ratio: 0.1,
            idempotent_only: true,
        }
    }
}

impl RetryPolicy {
    /// 创建指数重试策略
    pub fn exponential(max_attempts: u32, base_delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms,
            ..Default::default()
        }
    }

    /// 创建固定延迟重试策略
    pub fn fixed(max_attempts: u32, delay_ms: u64) -> Self {
        Self {
            max_attempts,
            base_delay_ms: delay_ms,
            max_delay_ms: delay_ms,
            exponential_base: 1.0,
            jitter_ratio: 0.0,
            idempotent_only: true,
        }
    }

    /// 计算重试延迟时间
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }

        // 指数退避: base_delay * exponential_base^(attempt-1)
        let exponential_delay = self.base_delay_ms as f64 
            * self.exponential_base.powi((attempt - 1) as i32);
        
        // 应用最大延迟限制
        let capped_delay = exponential_delay.min(self.max_delay_ms as f64);
        
        // 添加随机抖动
        let jitter = if self.jitter_ratio > 0.0 {
            let max_jitter = capped_delay * self.jitter_ratio;
            fastrand::f64() * max_jitter
        } else {
            0.0
        };

        Duration::from_millis((capped_delay + jitter) as u64)
    }

    /// 判断HTTP状态码是否应该重试
    pub fn should_retry_status(&self, status: u16) -> bool {
        match status {
            // 5xx 服务器错误 - 应该重试
            500..=599 => true,
            // 429 限流 - 应该重试
            429 => true,
            // 408 请求超时 - 应该重试
            408 => true,
            // 其他状态码不重试
            _ => false,
        }
    }

    /// 判断HTTP方法是否幂等
    pub fn is_idempotent_method(method: &crate::types::http::HttpMethod) -> bool {
        use crate::types::http::HttpMethod;
        match method {
            HttpMethod::Get | HttpMethod::Put | HttpMethod::Delete => true,
            HttpMethod::Post => false,
        }
    }
}

/// 重试配置解析结果
#[derive(Clone)]
pub struct RetryConfig {
    pub policy: RetryPolicy,
    pub raw_config: LitStr,
}

impl std::fmt::Debug for RetryConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RetryConfig")
            .field("policy", &self.policy)
            .field("raw_config", &self.raw_config.value())
            .finish()
    }
}

impl RetryConfig {
    /// 从字符串解析重试配置
    /// 
    /// 支持格式:
    /// - "exponential(max_attempts=3, base_delay=100ms)"
    /// - "fixed(max_attempts=5, delay=200ms)"
    /// - "exponential(3, 100ms)" // 简化格式
    pub fn parse(config_str: &LitStr) -> Result<Self, syn::Error> {
        let config_value = config_str.value();
        let policy = Self::parse_policy_string(&config_value)
            .map_err(|msg| syn::Error::new(config_str.span(), msg))?;

        Ok(RetryConfig {
            policy,
            raw_config: config_str.clone(),
        })
    }

    fn parse_policy_string(config: &str) -> Result<RetryPolicy, String> {
        let config = config.trim();
        
        if config.starts_with("exponential(") && config.ends_with(')') {
            Self::parse_exponential_config(&config[12..config.len()-1])
        } else if config.starts_with("fixed(") && config.ends_with(')') {
            Self::parse_fixed_config(&config[6..config.len()-1])
        } else {
            Err(format!("Unsupported retry config format: {}", config))
        }
    }

    fn parse_exponential_config(params: &str) -> Result<RetryPolicy, String> {
        let mut policy = RetryPolicy::default();
        
        // 解析参数
        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() { continue; }
            
            if let Some((key, value)) = param.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "max_attempts" => {
                        policy.max_attempts = value.parse()
                            .map_err(|_| format!("Invalid max_attempts: {}", value))?;
                    }
                    "base_delay" => {
                        policy.base_delay_ms = Self::parse_duration(value)?;
                    }
                    "max_delay" => {
                        policy.max_delay_ms = Self::parse_duration(value)?;
                    }
                    "exponential_base" => {
                        policy.exponential_base = value.parse()
                            .map_err(|_| format!("Invalid exponential_base: {}", value))?;
                    }
                    "jitter_ratio" => {
                        policy.jitter_ratio = value.parse()
                            .map_err(|_| format!("Invalid jitter_ratio: {}", value))?;
                    }
                    "idempotent_only" => {
                        policy.idempotent_only = value.parse()
                            .map_err(|_| format!("Invalid idempotent_only: {}", value))?;
                    }
                    _ => return Err(format!("Unknown parameter: {}", key)),
                }
            } else {
                // 简化格式：exponential(3, 100ms)
                let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 1 {
                    policy.max_attempts = parts[0].parse()
                        .map_err(|_| format!("Invalid max_attempts: {}", parts[0]))?;
                }
                if parts.len() >= 2 {
                    policy.base_delay_ms = Self::parse_duration(parts[1])?;
                }
                break;
            }
        }
        
        Ok(policy)
    }

    fn parse_fixed_config(params: &str) -> Result<RetryPolicy, String> {
        let mut policy = RetryPolicy::default();
        policy.exponential_base = 1.0; // 固定延迟

        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() { continue; }
            
            if let Some((key, value)) = param.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "max_attempts" => {
                        policy.max_attempts = value.parse()
                            .map_err(|_| format!("Invalid max_attempts: {}", value))?;
                    }
                    "delay" => {
                        let delay = Self::parse_duration(value)?;
                        policy.base_delay_ms = delay;
                        policy.max_delay_ms = delay;
                    }
                    _ => return Err(format!("Unknown parameter: {}", key)),
                }
            }
        }
        
        Ok(policy)
    }

    fn parse_duration(duration_str: &str) -> Result<u64, String> {
        let duration_str = duration_str.trim();
        
        if duration_str.ends_with("ms") {
            duration_str[..duration_str.len()-2].parse()
                .map_err(|_| format!("Invalid milliseconds: {}", duration_str))
        } else if duration_str.ends_with('s') {
            let seconds: u64 = duration_str[..duration_str.len()-1].parse()
                .map_err(|_| format!("Invalid seconds: {}", duration_str))?;
            Ok(seconds * 1000)
        } else {
            // 默认按毫秒处理
            duration_str.parse()
                .map_err(|_| format!("Invalid duration (expected ms or s suffix): {}", duration_str))
        }
    }
}

#[cfg(test)]
mod basic_tests {
    use super::*;
    use syn::{LitStr, parse_quote};

    #[test]
    fn test_retry_policy_default() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.base_delay_ms, 100);
        assert_eq!(policy.exponential_base, 2.0);
    }

    #[test]
    fn test_calculate_delay() {
        let policy = RetryPolicy::exponential(3, 100);
        
        assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));
        assert!(policy.calculate_delay(1).as_millis() >= 100);
        assert!(policy.calculate_delay(2).as_millis() >= 200);
        assert!(policy.calculate_delay(3).as_millis() >= 400);
    }

    #[test]
    fn test_should_retry_status() {
        let policy = RetryPolicy::default();
        
        assert!(policy.should_retry_status(500));
        assert!(policy.should_retry_status(502));
        assert!(policy.should_retry_status(429));
        assert!(policy.should_retry_status(408));
        
        assert!(!policy.should_retry_status(200));
        assert!(!policy.should_retry_status(400));
        assert!(!policy.should_retry_status(404));
    }

    #[test]
    fn test_parse_exponential_simple() {
        let config: LitStr = parse_quote! { "exponential(3, 100ms)" };
        let result = RetryConfig::parse(&config).unwrap();
        
        assert_eq!(result.policy.max_attempts, 3);
        assert_eq!(result.policy.base_delay_ms, 100);
        assert_eq!(result.policy.exponential_base, 2.0);
    }

    #[test]
    fn test_parse_exponential_detailed() {
        let config: LitStr = parse_quote! { "exponential(max_attempts=5, base_delay=200ms, max_delay=10s)" };
        let result = RetryConfig::parse(&config).unwrap();
        
        assert_eq!(result.policy.max_attempts, 5);
        assert_eq!(result.policy.base_delay_ms, 200);
        assert_eq!(result.policy.max_delay_ms, 10000);
    }

    #[test]
    fn test_parse_fixed() {
        let config: LitStr = parse_quote! { "fixed(max_attempts=3, delay=500ms)" };
        let result = RetryConfig::parse(&config).unwrap();
        
        assert_eq!(result.policy.max_attempts, 3);
        assert_eq!(result.policy.base_delay_ms, 500);
        assert_eq!(result.policy.exponential_base, 1.0);
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(RetryConfig::parse_duration("100ms").unwrap(), 100);
        assert_eq!(RetryConfig::parse_duration("2s").unwrap(), 2000);
        assert_eq!(RetryConfig::parse_duration("500").unwrap(), 500);
    }
}

// 引入详细测试模块
#[path = "retry_test.rs"]
mod comprehensive_retry_tests;