use std::any::TypeId;
use std::collections::HashMap;
use std::sync::Arc;
use crate::interceptor::SwanInterceptor;

/// 拦截器缓存管理器
/// 
/// 为每个客户端实例管理拦截器的生命周期，避免重复创建。
/// 使用 Arc 来实现零成本的拦截器共享。
pub struct InterceptorCache {
    /// 方法级拦截器缓存，按类型ID索引
    method_interceptors: HashMap<TypeId, Arc<dyn SwanInterceptor + Send + Sync>>,
}

impl InterceptorCache {
    /// 创建新的拦截器缓存
    pub fn new() -> Self {
        Self {
            method_interceptors: HashMap::new(),
        }
    }

    /// 获取或创建方法级拦截器
    /// 
    /// # 类型参数
    /// 
    /// * `T` - 拦截器类型，必须实现 SwanInterceptor + Default + Send + Sync
    /// 
    /// # 返回值
    /// 
    /// 返回拦截器的 Arc 引用
    pub fn get_or_create<T>(&mut self) -> Arc<dyn SwanInterceptor + Send + Sync>
    where
        T: SwanInterceptor + Default + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();
        
        self.method_interceptors
            .entry(type_id)
            .or_insert_with(|| Arc::new(T::default()))
            .clone()
    }

    /// 预热拦截器缓存
    /// 
    /// 在客户端初始化时调用，预先创建常用的拦截器实例
    pub fn warmup<T>(&mut self)
    where
        T: SwanInterceptor + Default + Send + Sync + 'static,
    {
        let _ = self.get_or_create::<T>();
    }

    /// 清空缓存（主要用于测试）
    #[cfg(test)]
    pub fn clear(&mut self) {
        self.method_interceptors.clear();
    }

    /// 获取缓存大小（用于监控）
    pub fn size(&self) -> usize {
        self.method_interceptors.len()
    }
}

impl Default for InterceptorCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interceptor::traits::NoOpInterceptor;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use async_trait::async_trait;
    use std::borrow::Cow;

    static CREATION_COUNT: AtomicUsize = AtomicUsize::new(0);

    #[derive(Default)]
    struct TestInterceptor;

    #[async_trait]
    impl SwanInterceptor for TestInterceptor {
        async fn before_request<'a>(
            &self,
            request: reqwest::RequestBuilder,
            request_body: &'a [u8],
            _context: Option<&(dyn std::any::Any + Send + Sync)>,
        ) -> anyhow::Result<(reqwest::RequestBuilder, Cow<'a, [u8]>)> {
            CREATION_COUNT.fetch_add(1, Ordering::SeqCst);
            Ok((request, Cow::Borrowed(request_body)))
        }

        async fn after_response(
            &self,
            response: reqwest::Response,
            _context: Option<&(dyn std::any::Any + Send + Sync)>,
        ) -> anyhow::Result<reqwest::Response> {
            Ok(response)
        }
    }

    #[test]
    fn test_cache_reuses_interceptors() {
        let mut cache = InterceptorCache::new();
        
        // 重置计数器
        CREATION_COUNT.store(0, Ordering::SeqCst);
        
        // 多次获取同一类型的拦截器
        let interceptor1 = cache.get_or_create::<TestInterceptor>();
        let interceptor2 = cache.get_or_create::<TestInterceptor>();
        let interceptor3 = cache.get_or_create::<TestInterceptor>();
        
        // 验证是同一个实例（Arc 指针相同）
        assert!(Arc::ptr_eq(&interceptor1, &interceptor2));
        assert!(Arc::ptr_eq(&interceptor2, &interceptor3));
        
        // 验证缓存大小
        assert_eq!(cache.size(), 1);
    }

    #[test]
    fn test_cache_different_types() {
        let mut cache = InterceptorCache::new();
        
        let _interceptor1 = cache.get_or_create::<TestInterceptor>();
        let _interceptor2 = cache.get_or_create::<NoOpInterceptor>();
        
        // 不同类型应该创建不同的实例
        assert_eq!(cache.size(), 2);
    }

    #[test]
    fn test_warmup() {
        let mut cache = InterceptorCache::new();
        
        // 预热缓存
        cache.warmup::<TestInterceptor>();
        cache.warmup::<NoOpInterceptor>();
        
        assert_eq!(cache.size(), 2);
        
        // 后续获取应该直接返回缓存的实例
        let _interceptor = cache.get_or_create::<TestInterceptor>();
        assert_eq!(cache.size(), 2); // 大小不变
    }
}