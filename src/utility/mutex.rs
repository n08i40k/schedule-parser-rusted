use std::ops::DerefMut;
use std::sync::Mutex;

pub trait MutexScope<T, ScopeFn, ScopeFnOutput>
where
    ScopeFn: FnOnce(&mut T) -> ScopeFnOutput,
{
    /// Replaces manually creating a mutex lock to perform operations on the data it manages.
    ///
    /// # Arguments
    ///
    /// * `f`: Function (mostly lambda) to which a reference to the mutable object stored in the mutex will be passed.
    ///
    /// returns: Return value of `f` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mtx: Mutex<i32> = Mutex::new(10);
    ///
    /// let res = mtx.scope(|x| { *x = *x * 2; *x });
    /// assert_eq!(res, *mtx.lock().unwrap());
    /// ```
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
    /// ## Asynchronous variant of [MutexScope::scope][MutexScope::scope].
    ///
    /// Replaces manually creating a mutex lock to perform operations on the data it manages.
    ///
    /// # Arguments
    ///
    /// * `f`: Asynchronous function (mostly lambda) to which a reference to the mutable object stored in the mutex will be passed.
    ///
    /// returns: Return value of `f` function.
    ///
    /// # Examples
    ///
    /// ```
    /// let mtx: Mutex<i32> = Mutex::new(10);
    ///
    /// let res = mtx.async_scope(async |x| { *x = *x * 2; *x }).await;
    /// assert_eq!(res, *mtx.lock().unwrap());
    /// ```
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
