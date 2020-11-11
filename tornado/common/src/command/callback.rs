use crate::command::{Command, CommandMut};
use std::future::Future;
use std::marker::PhantomData;

/// Command that executes a Fn callback
pub struct CallbackCommand<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> {
    callback: F,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> CallbackCommand<F, Fut, I, O> {
    pub fn new(callback: F) -> Self {
        Self { callback, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: Fn(I) -> Fut, Fut: Future<Output = O>, I, O> Command<I, O>
    for CallbackCommand<F, Fut, I, O>
{
    async fn execute(&self, message: I) -> O {
        (self.callback)(message).await
    }
}

/// Command that executes a FnMut callback
pub struct CallbackCommandMut<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> {
    callback: F,
    phantom_i: PhantomData<I>,
    phantom_o: PhantomData<O>,
}

impl<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> CallbackCommandMut<F, Fut, I, O> {
    pub fn new(callback: F) -> Self {
        Self { callback, phantom_i: PhantomData, phantom_o: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<F: FnMut(I) -> Fut, Fut: Future<Output = O>, I, O> CommandMut<I, O>
    for CallbackCommandMut<F, Fut, I, O>
{
    async fn execute(&mut self, message: I) -> O {
        (self.callback)(message).await
    }
}
