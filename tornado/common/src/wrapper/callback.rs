use crate::wrapper::{Wrapper, WrapperMut};
use std::future::Future;
use std::marker::PhantomData;

/// Wrapper for a Fn callback
pub struct CallbackWrapper<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> {
    callback: F,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> CallbackWrapper<F, Fut, I, O> {
    pub fn new(callback: F) -> Self {
        Self { callback, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> Wrapper<I, O>
    for CallbackWrapper<F, Fut, I, O>
{
    async fn execute(&self, message: I) -> O {
        (self.callback)(message).await
    }
}

/// Wrapper for a FnMut callback
pub struct CallbackWrapperMut<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> {
    callback: F,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> CallbackWrapperMut<F, Fut, I, O> {
    pub fn new(callback: F) -> Self {
        Self { callback, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> WrapperMut<I, O>
    for CallbackWrapperMut<F, Fut, I, O>
{
    async fn execute(&mut self, message: I) -> O {
        (self.callback)(message).await
    }
}
