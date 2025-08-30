#[cfg(test)]
mod comprehensive_tests {
    use super::*;
    use crate::{RetryPolicy, RetryConfig, HttpMethod};
    use syn::{LitStr, parse_quote};
    use std::time::Duration;

    /// 测试重试策略的核心算法
    mod retry_policy_tests {
        use super::*;

        #[test]
        fn test_exponential_backoff_calculation() {
            let policy = RetryPolicy {
                max_attempts: 5,
                base_delay_ms: 100,
                max_delay_ms: 10000,
                exponential_base: 2.0,
                jitter_ratio: 0.0, // 无抖动便于测试
                idempotent_only: true,
            };

            // 测试指数增长
            assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));
            assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));  // 100 * 2^0
            assert_eq!(policy.calculate_delay(2), Duration::from_millis(200));  // 100 * 2^1
            assert_eq!(policy.calculate_delay(3), Duration::from_millis(400));  // 100 * 2^2
            assert_eq!(policy.calculate_delay(4), Duration::from_millis(800));  // 100 * 2^3
        }

        #[test]
        fn test_max_delay_cap() {
            let policy = RetryPolicy {
                max_attempts: 10,
                base_delay_ms: 1000,
                max_delay_ms: 5000,
                exponential_base: 2.0,
                jitter_ratio: 0.0,
                idempotent_only: true,
            };

            // 第6次重试应该达到上限
            let delay_6 = policy.calculate_delay(6); // 1000 * 2^5 = 32000ms
            assert_eq!(delay_6, Duration::from_millis(5000)); // 被限制在5000ms
        }

        #[test]
        fn test_jitter_effect() {
            let policy = RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 1000,
                max_delay_ms: 10000,
                exponential_base: 2.0,
                jitter_ratio: 0.5, // 50%抖动
                idempotent_only: true,
            };

            // 多次测试确保抖动在合理范围内
            for _ in 0..10 {
                let delay = policy.calculate_delay(2);
                let delay_ms = delay.as_millis() as f64;
                
                // 基础延迟是 1000 * 2^1 = 2000ms
                // 抖动范围是 [2000, 2000 + 2000*0.5] = [2000, 3000]
                assert!(delay_ms >= 2000.0);
                assert!(delay_ms <= 3000.0);
            }
        }

        #[test]
        fn test_fixed_delay_policy() {
            let policy = RetryPolicy::fixed(4, 500);
            
            // 固定延迟不应该增长
            assert_eq!(policy.calculate_delay(1), Duration::from_millis(500));
            assert_eq!(policy.calculate_delay(2), Duration::from_millis(500));
            assert_eq!(policy.calculate_delay(3), Duration::from_millis(500));
            assert_eq!(policy.exponential_base, 1.0);
        }

        #[test]
        fn test_retry_status_conditions() {
            let policy = RetryPolicy::default();

            // 应该重试的状态码
            assert!(policy.should_retry_status(500)); // 内部服务器错误
            assert!(policy.should_retry_status(502)); // 网关错误
            assert!(policy.should_retry_status(503)); // 服务不可用
            assert!(policy.should_retry_status(504)); // 网关超时
            assert!(policy.should_retry_status(429)); // 限流
            assert!(policy.should_retry_status(408)); // 请求超时

            // 不应该重试的状态码
            assert!(!policy.should_retry_status(200)); // 成功
            assert!(!policy.should_retry_status(400)); // 客户端错误
            assert!(!policy.should_retry_status(401)); // 未授权
            assert!(!policy.should_retry_status(403)); // 禁止访问
            assert!(!policy.should_retry_status(404)); // 未找到
            assert!(!policy.should_retry_status(422)); // 无法处理的实体
        }

        #[test]
        fn test_idempotent_method_detection() {

            // 幂等方法
            assert!(RetryPolicy::is_idempotent_method(&HttpMethod::Get));
            assert!(RetryPolicy::is_idempotent_method(&HttpMethod::Put));
            assert!(RetryPolicy::is_idempotent_method(&HttpMethod::Delete));

            // 非幂等方法
            assert!(!RetryPolicy::is_idempotent_method(&HttpMethod::Post));
        }
    }

    /// 测试重试配置解析
    mod retry_config_parsing_tests {
        use super::*;

        #[test]
        fn test_parse_simple_exponential() {
            let config: LitStr = parse_quote! { "exponential(3, 100ms)" };
            let result = RetryConfig::parse(&config).unwrap();
            
            assert_eq!(result.policy.max_attempts, 3);
            assert_eq!(result.policy.base_delay_ms, 100);
            assert_eq!(result.policy.exponential_base, 2.0);
            assert_eq!(result.policy.jitter_ratio, 0.1);
            assert!(result.policy.idempotent_only);
        }

        #[test]
        fn test_parse_detailed_exponential() {
            let config: LitStr = parse_quote! { 
                "exponential(max_attempts=7, base_delay=250ms, max_delay=30s, exponential_base=1.5, jitter_ratio=0.3)" 
            };
            let result = RetryConfig::parse(&config).unwrap();
            
            assert_eq!(result.policy.max_attempts, 7);
            assert_eq!(result.policy.base_delay_ms, 250);
            assert_eq!(result.policy.max_delay_ms, 30000);
            assert_eq!(result.policy.exponential_base, 1.5);
            assert_eq!(result.policy.jitter_ratio, 0.3);
        }

        #[test]
        fn test_parse_fixed_delay() {
            let config: LitStr = parse_quote! { "fixed(max_attempts=5, delay=2s)" };
            let result = RetryConfig::parse(&config).unwrap();
            
            assert_eq!(result.policy.max_attempts, 5);
            assert_eq!(result.policy.base_delay_ms, 2000);
            assert_eq!(result.policy.max_delay_ms, 2000);
            assert_eq!(result.policy.exponential_base, 1.0);
        }

        #[test]
        fn test_parse_with_non_idempotent() {
            let config: LitStr = parse_quote! { "exponential(max_attempts=3, base_delay=100ms, idempotent_only=false)" };
            let result = RetryConfig::parse(&config).unwrap();
            
            assert_eq!(result.policy.max_attempts, 3);
            assert!(!result.policy.idempotent_only);
        }

        #[test]
        fn test_parse_duration_formats() {
            // 毫秒格式
            assert_eq!(RetryConfig::parse_duration("100ms").unwrap(), 100);
            assert_eq!(RetryConfig::parse_duration("1500ms").unwrap(), 1500);
            
            // 秒格式
            assert_eq!(RetryConfig::parse_duration("1s").unwrap(), 1000);
            assert_eq!(RetryConfig::parse_duration("5s").unwrap(), 5000);
            
            // 纯数字（默认毫秒）
            assert_eq!(RetryConfig::parse_duration("300").unwrap(), 300);
        }

        #[test]
        fn test_parse_invalid_configs() {
            // 无效格式
            let invalid1: LitStr = parse_quote! { "invalid_format" };
            assert!(RetryConfig::parse(&invalid1).is_err());
            
            let invalid2: LitStr = parse_quote! { "exponential(invalid_number)" };
            assert!(RetryConfig::parse(&invalid2).is_err());
            
            let invalid3: LitStr = parse_quote! { "exponential(max_attempts=abc)" };
            assert!(RetryConfig::parse(&invalid3).is_err());
        }

        #[test]
        fn test_parse_edge_cases() {
            // 最小配置
            let min_config: LitStr = parse_quote! { "exponential(1, 1ms)" };
            let result = RetryConfig::parse(&min_config).unwrap();
            assert_eq!(result.policy.max_attempts, 1);
            assert_eq!(result.policy.base_delay_ms, 1);

            // 最大合理配置
            let max_config: LitStr = parse_quote! { "exponential(max_attempts=100, base_delay=1ms, max_delay=3600s)" };
            let result = RetryConfig::parse(&max_config).unwrap();
            assert_eq!(result.policy.max_attempts, 100);
            assert_eq!(result.policy.max_delay_ms, 3600000); // 1小时
        }
    }

    /// 测试重试时间计算的数学正确性
    mod mathematical_correctness_tests {
        use super::*;

        #[test]
        fn test_exponential_progression() {
            let policy = RetryPolicy::exponential(10, 50);
            
            let delays: Vec<u64> = (1..=5)
                .map(|attempt| policy.calculate_delay(attempt).as_millis() as u64)
                .collect();
            
            // 验证指数增长趋势（允许抖动误差）
            for i in 1..delays.len() {
                let ratio = delays[i] as f64 / delays[i-1] as f64;
                assert!(ratio >= 1.5 && ratio <= 2.5, 
                       "Exponential growth ratio should be ~2.0, got {:.2}", ratio);
            }
        }

        #[test]
        fn test_cumulative_delay_bounds() {
            let policy = RetryPolicy {
                max_attempts: 5,
                base_delay_ms: 100,
                max_delay_ms: 2000,
                exponential_base: 2.0,
                jitter_ratio: 0.1,
                idempotent_only: true,
            };

            let total_delay: u64 = (1..=5)
                .map(|attempt| policy.calculate_delay(attempt).as_millis() as u64)
                .sum();

            // 总延迟应该在合理范围内 (理论值约为100+200+400+800+1600+抖动)
            assert!(total_delay >= 3000); // 最小理论值
            assert!(total_delay <= 5000); // 最大合理值（包含抖动和cap）
        }

        #[test]
        fn test_jitter_distribution() {
            let policy = RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 1000,
                max_delay_ms: 10000,
                exponential_base: 2.0,
                jitter_ratio: 0.2, // 20%抖动
                idempotent_only: true,
            };

            let mut delays = Vec::new();
            for _ in 0..100 {
                delays.push(policy.calculate_delay(2).as_millis() as f64);
            }

            // 统计分析
            let min_delay = delays.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_delay = delays.iter().fold(0.0f64, |a, &b| a.max(b));
            let avg_delay = delays.iter().sum::<f64>() / delays.len() as f64;

            // 基础延迟是2000ms，抖动范围[2000, 2400]
            assert!(min_delay >= 2000.0);
            assert!(max_delay <= 2400.0);
            assert!(avg_delay >= 2100.0 && avg_delay <= 2300.0);
        }
    }

    /// 边界条件和错误情况测试
    mod edge_cases_tests {
        use super::*;

        #[test]
        fn test_zero_attempts() {
            let policy = RetryPolicy {
                max_attempts: 0,
                ..Default::default()
            };
            
            // 0次重试应该立即返回
            assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));
            assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));
        }

        #[test]
        fn test_extreme_exponential_base() {
            let policy = RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 10,
                max_delay_ms: 1000,
                exponential_base: 10.0, // 极大的指数底数
                jitter_ratio: 0.0,
                idempotent_only: true,
            };

            let delay_1 = policy.calculate_delay(1).as_millis(); // 10ms
            let delay_2 = policy.calculate_delay(2).as_millis(); // 100ms
            let delay_3 = policy.calculate_delay(3).as_millis(); // 1000ms (capped)

            assert_eq!(delay_1, 10);
            assert_eq!(delay_2, 100);
            assert_eq!(delay_3, 1000);
        }

        #[test]
        fn test_fractional_exponential_base() {
            let policy = RetryPolicy {
                max_attempts: 4,
                base_delay_ms: 1000,
                max_delay_ms: 10000,
                exponential_base: 1.5, // 较温和的增长
                jitter_ratio: 0.0,
                idempotent_only: true,
            };

            assert_eq!(policy.calculate_delay(1).as_millis(), 1000);  // 1000 * 1.5^0
            assert_eq!(policy.calculate_delay(2).as_millis(), 1500);  // 1000 * 1.5^1
            assert_eq!(policy.calculate_delay(3).as_millis(), 2250);  // 1000 * 1.5^2
        }

        #[test]
        fn test_boundary_status_codes() {
            let policy = RetryPolicy::default();

            // 边界状态码测试
            assert!(!policy.should_retry_status(499)); // 4xx边界
            assert!(policy.should_retry_status(500));  // 5xx开始
            assert!(policy.should_retry_status(599));  // 5xx结束
            assert!(!policy.should_retry_status(600)); // 超出5xx范围

            // 特殊状态码
            assert!(policy.should_retry_status(429));  // 限流
            assert!(policy.should_retry_status(408));  // 超时
            assert!(!policy.should_retry_status(407)); // 代理认证（不重试）
            assert!(!policy.should_retry_status(409)); // 冲突（不重试）
        }
    }

    /// 配置解析的详细测试
    mod config_parsing_edge_cases {
        use super::*;

        #[test]
        fn test_whitespace_handling() {
            let configs = vec![
                "exponential(3, 100ms)",           // 无空格
                "exponential( 3 , 100ms )",        // 有空格
                "exponential( 3, 100ms)",          // 混合空格
                "exponential(3 ,100ms )",          // 各种空格组合
            ];

            for config_str in configs {
                let config: LitStr = LitStr::new(config_str, proc_macro2::Span::call_site());
                let result = RetryConfig::parse(&config);
                assert!(result.is_ok(), "Failed to parse: {}", config_str);
                assert_eq!(result.unwrap().policy.max_attempts, 3);
            }
        }

        #[test]
        fn test_parameter_order_independence() {
            let config1: LitStr = parse_quote! { "exponential(max_attempts=3, base_delay=100ms)" };
            let config2: LitStr = parse_quote! { "exponential(base_delay=100ms, max_attempts=3)" };
            
            let result1 = RetryConfig::parse(&config1).unwrap();
            let result2 = RetryConfig::parse(&config2).unwrap();
            
            assert_eq!(result1.policy.max_attempts, result2.policy.max_attempts);
            assert_eq!(result1.policy.base_delay_ms, result2.policy.base_delay_ms);
        }

        #[test]
        fn test_mixed_parameter_formats() {
            // 简化 + 命名参数混合
            let config: LitStr = parse_quote! { "exponential(5, 200ms, max_delay=30s, jitter_ratio=0.15)" };
            let result = RetryConfig::parse(&config).unwrap();
            
            assert_eq!(result.policy.max_attempts, 5);
            assert_eq!(result.policy.base_delay_ms, 200);
            assert_eq!(result.policy.max_delay_ms, 30000);
            assert_eq!(result.policy.jitter_ratio, 0.15);
        }

        #[test]
        fn test_duration_parsing_variants() {
            assert_eq!(RetryConfig::parse_duration("0ms").unwrap(), 0);
            assert_eq!(RetryConfig::parse_duration("999ms").unwrap(), 999);
            assert_eq!(RetryConfig::parse_duration("1s").unwrap(), 1000);
            assert_eq!(RetryConfig::parse_duration("60s").unwrap(), 60000);
            assert_eq!(RetryConfig::parse_duration("500").unwrap(), 500);

            // 无效格式
            assert!(RetryConfig::parse_duration("100x").is_err());
            assert!(RetryConfig::parse_duration("abc").is_err());
            assert!(RetryConfig::parse_duration("").is_err());
        }

        #[test]
        fn test_malformed_configs() {
            let malformed_configs = vec![
                "exponential(",               // 不完整
                "exponential)",               // 缺少开括号
                "exponential(,)",            // 空参数
                "exponential(3,)",           // 缺少参数
                "exponential(3, 100)",       // 缺少单位
                "exponential(max_attempts)",  // 缺少值
                "fixed(delay=100ms)",        // 缺少required参数
                "unknown_strategy(3, 100ms)", // 未知策略
            ];

            for config_str in malformed_configs {
                let config: LitStr = LitStr::new(config_str, proc_macro2::Span::call_site());
                let result = RetryConfig::parse(&config);
                assert!(result.is_err(), "Should fail for: {}", config_str);
            }
        }
    }

    /// 性能和内存使用测试
    mod performance_tests {
        use super::*;

        #[test]
        fn test_policy_memory_footprint() {
            let policy = RetryPolicy::default();
            let size = std::mem::size_of_val(&policy);
            
            // 重试策略应该是轻量级的
            assert!(size <= 64, "RetryPolicy size {} bytes should be ≤ 64 bytes", size);
        }

        #[test]
        fn test_delay_calculation_performance() {
            let policy = RetryPolicy::exponential(10, 100);
            
            let start = std::time::Instant::now();
            for attempt in 1..=1000 {
                let _ = policy.calculate_delay(attempt % 10);
            }
            let duration = start.elapsed();
            
            // 1000次计算应该在1ms内完成
            assert!(duration.as_millis() < 10, 
                   "Delay calculation too slow: {}ms for 1000 operations", 
                   duration.as_millis());
        }

        #[test]
        fn test_config_parsing_performance() {
            let config: LitStr = parse_quote! { "exponential(max_attempts=5, base_delay=100ms, max_delay=10s)" };
            
            let start = std::time::Instant::now();
            for _ in 0..100 {
                let _ = RetryConfig::parse(&config).unwrap();
            }
            let duration = start.elapsed();
            
            // 100次解析应该很快
            assert!(duration.as_millis() < 100, 
                   "Config parsing too slow: {}ms for 100 operations", 
                   duration.as_millis());
        }
    }

    /// 实际场景模拟测试
    mod scenario_simulation_tests {
        use super::*;

        #[test]
        fn test_cascading_failure_scenario() {
            // 模拟级联故障场景的重试行为
            let policy = RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 100,
                max_delay_ms: 1000,
                exponential_base: 2.0,
                jitter_ratio: 0.1,
                idempotent_only: true,
            };

            // 模拟连续失败的状态码序列
            let failure_statuses = vec![503, 502, 500]; // 服务不可用 -> 网关错误 -> 内部错误
            
            for &status in &failure_statuses {
                assert!(policy.should_retry_status(status), 
                       "Should retry status {} in cascading failure", status);
            }
        }

        #[test]
        fn test_rate_limiting_scenario() {
            let policy = RetryPolicy {
                max_attempts: 5,
                base_delay_ms: 1000, // 较长的基础延迟应对限流
                max_delay_ms: 60000, // 1分钟最大延迟
                exponential_base: 1.5, // 较温和的增长
                jitter_ratio: 0.3, // 较大的抖动
                idempotent_only: true,
            };

            assert!(policy.should_retry_status(429)); // 限流状态应该重试
            
            // 验证限流场景下的延迟序列
            let delays: Vec<u64> = (1..=3)
                .map(|attempt| policy.calculate_delay(attempt).as_millis() as u64)
                .collect();
            
            // 在限流场景下，延迟应该相对较长且温和增长
            assert!(delays[0] >= 1000); // 至少1秒
            assert!(delays[1] >= 1400); // 温和增长
            assert!(delays[2] >= 2000); // 继续增长
        }

        #[test]
        fn test_microservice_timeout_scenario() {
            // 微服务超时场景：快速重试，有限次数
            let policy = RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 50,   // 快速重试
                max_delay_ms: 500,   // 较短的最大延迟
                exponential_base: 2.0,
                jitter_ratio: 0.0,  // 无抖动，可预测的时间
                idempotent_only: true,
            };

            let total_retry_time: u64 = (1..=3)
                .map(|attempt| policy.calculate_delay(attempt).as_millis() as u64)
                .sum();

            // 总重试时间应该很短，适合微服务架构
            assert!(total_retry_time <= 1000, 
                   "Microservice retry should be fast, got {}ms", total_retry_time);
        }
    }
}