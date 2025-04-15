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

pub trait MutexScopeAsync<T> {
    async fn async_scope<'a, F, FnFut, FnOut>(&'a self, f: F) -> FnOut
    where
        FnFut: Future<Output = FnOut>,
        F: FnOnce(&'a mut T) -> FnFut,
        T: 'a;
}

impl<T> MutexScopeAsync<T> for Mutex<T> {
    async fn async_scope<'a, F, FnFut, FnOut>(&'a self, f: F) -> FnOut
    where
        FnFut: Future<Output = FnOut>,
        F: FnOnce(&'a mut T) -> FnFut,
        T: 'a,
    {
        let mut guard = self.lock().unwrap();

        let ptr: &'a mut T = unsafe { &mut *(guard.deref_mut() as *mut _) };
        f(ptr).await
    }
}