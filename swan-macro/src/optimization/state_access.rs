use quote::quote;

/// 状态访问性能优化器
/// 
/// 优化状态访问模式，使用 RwLock 提升并发读取性能
pub struct StateAccessOptimizer;

impl StateAccessOptimizer {
    /// 生成高性能状态访问代码
    /// 
    /// 使用 RwLock 替代 Mutex，支持并发读取访问，
    /// 减少状态访问的锁竞争
    pub fn generate_optimized_state_access() -> proc_macro2::TokenStream {
        quote! {
            /// 高性能状态容器
            /// 
            /// 使用 RwLock 支持多读者单写者模式，优化并发访问性能
            pub struct OptimizedStateContainer<T> {
                inner: std::sync::Arc<std::sync::RwLock<T>>,
            }
            
            impl<T> OptimizedStateContainer<T> {
                pub fn new(state: T) -> Self {
                    Self {
                        inner: std::sync::Arc::new(std::sync::RwLock::new(state)),
                    }
                }
                
                /// 获取状态的只读访问
                /// 
                /// 支持并发读取，不会阻塞其他读取操作
                #[inline(always)]
                pub fn read(&self) -> std::sync::RwLockReadGuard<T> {
                    self.inner.read().unwrap()
                }
                
                /// 获取状态的可写访问
                /// 
                /// 独占访问，会阻塞其他所有操作
                #[inline(always)]
                pub fn write(&self) -> std::sync::RwLockWriteGuard<T> {
                    self.inner.write().unwrap()
                }
                
                /// 尝试获取只读访问（非阻塞）
                #[inline(always)]
                pub fn try_read(&self) -> Option<std::sync::RwLockReadGuard<T>> {
                    self.inner.try_read().ok()
                }
                
                /// 尝试获取可写访问（非阻塞）
                #[inline(always)]
                pub fn try_write(&self) -> Option<std::sync::RwLockWriteGuard<T>> {
                    self.inner.try_write().ok()
                }
                
                /// 获取状态引用用于拦截器上下文
                #[inline(always)]
                pub fn as_context_ref(&self) -> &(dyn std::any::Any + Send + Sync) {
                    self as &(dyn std::any::Any + Send + Sync)
                }
            }
            
            impl<T> Clone for OptimizedStateContainer<T> {
                fn clone(&self) -> Self {
                    Self {
                        inner: self.inner.clone(),
                    }
                }
            }
            
            unsafe impl<T: Send> Send for OptimizedStateContainer<T> {}
            unsafe impl<T: Send + Sync> Sync for OptimizedStateContainer<T> {}
        }
    }

    /// 生成状态感知的拦截器上下文代码
    /// 
    /// 优化状态传递到拦截器的性能
    pub fn generate_state_context_code(has_state: bool) -> proc_macro2::TokenStream {
        if has_state {
            quote! {
                #[inline(always)]
                fn create_context(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
                    // 零分配：直接返回状态引用
                    self.state.as_ref().map(|s| s.as_context_ref())
                }
            }
        } else {
            quote! {
                #[inline(always)]
                fn create_context(&self) -> Option<&(dyn std::any::Any + Send + Sync)> {
                    // 编译时优化：无状态，返回None
                    None
                }
            }
        }
    }

    /// 生成局部状态缓存代码
    /// 
    /// 在长时间运行的操作中缓存状态访问，避免重复锁获取
    pub fn generate_local_state_cache() -> proc_macro2::TokenStream {
        quote! {
            /// 局部状态缓存
            /// 
            /// 在单个请求处理期间缓存状态访问，避免重复锁操作
            pub struct LocalStateCache<'a, T> {
                cached_read: Option<std::sync::RwLockReadGuard<'a, T>>,
                container: &'a OptimizedStateContainer<T>,
            }
            
            impl<'a, T> LocalStateCache<'a, T> {
                pub fn new(container: &'a OptimizedStateContainer<T>) -> Self {
                    Self {
                        cached_read: None,
                        container,
                    }
                }
                
                /// 获取或缓存状态读取锁
                #[inline(always)]
                pub fn get_or_cache(&mut self) -> &T {
                    if self.cached_read.is_none() {
                        self.cached_read = Some(self.container.read());
                    }
                    self.cached_read.as_ref().unwrap()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_state_access_generation() {
        let result = StateAccessOptimizer::generate_optimized_state_access();
        let result_str = result.to_string();
        assert!(result_str.contains("OptimizedStateContainer"));
        assert!(result_str.contains("RwLock"));
        assert!(result_str.contains("inline(always)"));
    }

    #[test]
    fn test_state_context_with_state() {
        let result = StateAccessOptimizer::generate_state_context_code(true);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_STATE: bool = true"));
        assert!(result_str.contains("as_context_ref"));
    }

    #[test]
    fn test_state_context_no_state() {
        let result = StateAccessOptimizer::generate_state_context_code(false);
        let result_str = result.to_string();
        assert!(result_str.contains("HAS_STATE: bool = false"));
        assert!(result_str.contains("None"));
    }

    #[test]
    fn test_local_state_cache_generation() {
        let result = StateAccessOptimizer::generate_local_state_cache();
        let result_str = result.to_string();
        assert!(result_str.contains("LocalStateCache"));
        assert!(result_str.contains("get_or_cache"));
    }
}