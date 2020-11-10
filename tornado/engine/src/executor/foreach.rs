use crate::executor::ActionMessage;
use actix::prelude::*;
use log::*;
use tornado_executor_common::{StatefulExecutor, ExecutorError};
use tornado_executor_foreach::ForEachExecutor;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForEachExecutorActorInitMessage<F: Fn() -> ForEachExecutor>
where
    F: Send + Sync,
{
    pub init: F,
}

pub struct ForEachExecutorActor {
    executor: Option<ForEachExecutor>,
}

impl ForEachExecutorActor {
    pub fn start_new(message_mailbox_capacity: usize) -> Addr<ForEachExecutorActor> {
        Self::create(move |ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            Self { executor: None }
        })
    }
}

impl Actor for ForEachExecutorActor {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("ForEachExecutorActor started.");
    }
}

impl Handler<ActionMessage> for ForEachExecutorActor {
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("ForEachExecutorActor - received new action [{:?}]", &msg.action);

        if let Some(executor) = &mut self.executor {
            match executor.execute(&msg.action) {
                Ok(_) => {
                    debug!("ForEachExecutorActor - {} - Action executed successfully", &executor);
                    Ok(())
                }
                Err(e) => {
                    error!(
                        "ForEachExecutorActor - {} - Failed to execute action: {}",
                        &executor, e
                    );
                    Err(e)
                }
            }
        } else {
            let message =
                "ForEachExecutorActor received a message when it was not yet initialized!"
                    .to_owned();
            error!("{}", message);
            Err(ExecutorError::ConfigurationError { message })
        }
    }
}

impl<F: Fn() -> ForEachExecutor> Handler<ForEachExecutorActorInitMessage<F>>
    for ForEachExecutorActor
where
    F: Send + Sync,
{
    type Result = ();

    fn handle(&mut self, msg: ForEachExecutorActorInitMessage<F>, _: &mut Context<Self>) {
        trace!("ForEachExecutorActor - received init message");
        self.executor = Some((msg.init)());
    }
}
