use actix::prelude::*;
use log::*;
use std::rc::Rc;
use tornado_common::actors::message::ActionMessage;
use tornado_executor_common::{ExecutorError, StatelessExecutor};
use tornado_executor_foreach::ForEachExecutor;
use tracing::Instrument;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ForEachExecutorActorInitMessage<F: Fn() -> ForEachExecutor>
where
    F: Send + Sync,
{
    pub init: F,
}

pub struct ForEachExecutorActor {
    executor: Option<Rc<ForEachExecutor>>,
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
        let parent_span = msg.0.span.clone();
        let _parent_guard = msg.0.span.entered();
        let action = msg.0.action;
        trace!("ForEachExecutorActor - received new action [{:?}]", &action);

        if let Some(executor) = &self.executor {
            let executor = executor.clone();
            actix::spawn(
                async move {
                    match executor.execute(action).await {
                        Ok(_) => {
                            debug!(
                                "ForEachExecutorActor - {} - Action executed successfully",
                                &executor
                            );
                        }
                        Err(e) => {
                            error!(
                                "ForEachExecutorActor - {} - Failed to execute action: {:?}",
                                &executor, e
                            );
                        }
                    }
                }
                .instrument(parent_span),
            );
            Ok(())
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
        self.executor = Some((msg.init)().into());
    }
}
