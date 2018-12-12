use actix::prelude::*;
use tornado_common_api::Action;
use tornado_executor_common::Executor;

#[derive(Message)]
pub struct ActionMessage {
    pub action: Action,
}

pub struct ExecutorActor<E: Executor> {
    pub action_id: String,
    pub executor: E,
}

impl <E: Executor + 'static> Actor for ExecutorActor<E>{
    type Context = SyncContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("ExecutorActor started.");
    }
}

impl <E: Executor + 'static> Handler<ActionMessage> for ExecutorActor<E> {
    type Result = ();

    fn handle(&mut self, msg: ActionMessage, _: &mut SyncContext<Self>) {
        debug!("ExecutorActor - received new action [{:?}]", &msg.action);
        match self.executor.execute(&msg.action) {
            Ok(_) => debug!("ExecutorActor - {} - Action executed successfully", self.action_id),
            Err(e) => error!("ExecutorActor - {} - Failed to execute action: {}", self.action_id, e),
        };
    }
}
