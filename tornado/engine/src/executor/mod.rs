use actix::prelude::*;
use log::*;
use std::fmt::Display;
use tornado_common_api::Action;
use tornado_executor_common::Executor;

pub mod icinga2;

#[derive(Message)]
pub struct ActionMessage {
    pub action: Action,
}

pub struct ExecutorActor<E: Executor + Display> {
    pub executor: E,
}

impl<E: Executor + Display + 'static> Actor for ExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ExecutorActor started.");
    }
}

impl<E: Executor + Display + 'static> Handler<ActionMessage> for ExecutorActor<E> {
    type Result = ();

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) {
        trace!("ExecutorActor - received new action [{:?}]", &msg.action);
        match self.executor.execute(msg.action) {
            Ok(_) => debug!("ExecutorActor - {} - Action executed successfully", &self.executor),
            Err(e) => {
                error!("ExecutorActor - {} - Failed to execute action: {}", &self.executor, e)
            }
        };
    }
}

#[derive(Message)]
pub struct LazyExecutorActorInitMessage<E: Executor + Display, F: Fn() -> E>
where
    F: Send + Sync,
{
    pub init: F,
}

pub struct LazyExecutorActor<E: Executor + Display> {
    pub executor: Option<E>,
}

impl<E: Executor + Display + 'static> Actor for LazyExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ExecutorActor started.");
    }
}

impl<E: Executor + Display + 'static> Handler<ActionMessage> for LazyExecutorActor<E> {
    type Result = ();

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) {
        trace!("LazyExecutorActor - received new action [{:?}]", &msg.action);

        if let Some(executor) = &mut self.executor {
            match executor.execute(msg.action) {
                Ok(_) => debug!("LazyExecutorActor - {} - Action executed successfully", &executor),
                Err(e) => {
                    error!("LazyExecutorActor - {} - Failed to execute action: {}", &executor, e)
                }
            };
        } else {
            error!("LazyExecutorActor received a message when it was not yet initialized!");
        }
    }
}

impl<E: Executor + Display + 'static, F: Fn() -> E> Handler<LazyExecutorActorInitMessage<E, F>>
    for LazyExecutorActor<E>
where
    F: Send + Sync,
{
    type Result = ();

    fn handle(&mut self, msg: LazyExecutorActorInitMessage<E, F>, _: &mut SyncContext<Self>) {
        trace!("LazyExecutorActor - received init message");
        self.executor = Some((msg.init)());
    }
}
