use actix::prelude::*;
use tornado_common_api::Action;
use tornado_executor_common::Executor;
use tornado_executor_common::ExecutorError;

pub mod archive;

#[derive(Message)]
pub struct ActionMessage {
    pub action: Action,
}

pub struct ExecutorActor<E: Executor> {
    pub action_id: String,
    pub executor: E,
}

impl <E: Executor> Actor for ExecutorActor<E> {
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ExecutorActor started.");
    }
}

impl <E: Executor> Handler<ActionMessage> for ExecutorActor<E> {
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) -> Self::Result {
        debug!("ExecutorActor - received new action [{:?}]", &msg.action);
        self.executor.execute(&msg.action)
    }
}
