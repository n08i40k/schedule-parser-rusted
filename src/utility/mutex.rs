use std::ops::DerefMut;
use std::sync::Mutex;

pub trait MutexScope<T, ScopeFn, ScopeFnOutput>
where
    ScopeFn: FnOnce(&mut T) -> ScopeFnOutput,
{
    fn scope(&self, f: ScopeFn) -> ScopeFnOutput;
}

impl<T, ScopeFn, ScopeFnOutput> MutexScope<T, ScopeFn, ScopeFnOutput> for Mutex<T>
where
    ScopeFn: FnOnce(&mut T) -> ScopeFnOutput,
{
    fn scope(&self, f: ScopeFn) -> ScopeFnOutput {
        let mut lock = self.lock().unwrap();
        let inner = lock.deref_mut();
        
        f(inner)
    }
}