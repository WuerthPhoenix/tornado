use crate::actors::message::ActionMessage;
use crate::command::Command;
use actix::{Actor, Addr, Context, Handler};
use log::*;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_executor_common::ExecutorError;
use tracing_futures::Instrument;

pub struct CommandExecutorActor<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> {
    pub command: Rc<T>,
}

impl<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> CommandExecutorActor<T> {
    pub fn start_new(
        message_mailbox_capacity: usize,
        command: Rc<T>,
    ) -> Addr<CommandExecutorActor<T>> {
        CommandExecutorActor::create(move |ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            Self { command }
        })
    }
}

impl<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> Actor
    for CommandExecutorActor<T>
{
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("CommandExecutorActor started.");
    }
}

impl<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> Handler<ActionMessage>
    for CommandExecutorActor<T>
{
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut Context<Self>) -> Self::Result {

        let trace_id = msg.action.trace_id.as_str();
        let span = tracing::error_span!("CommandExecutorActor", trace_id);

        let command = self.command.clone();

        actix::spawn(async move {
            let action = msg.action;
            let action_id = action.id.clone();
            trace!("CommandExecutorActor - received new action [{:?}]", &action);
            debug!("CommandExecutorActor - Execute action [{:?}]", &action_id);

            match command.execute(action).await {
                Ok(_) => {
                    debug!("CommandExecutorActor - Action [{}] executed successfully", &action_id);
                }
                Err(e) => {
                    error!("CommandExecutorActor - Failed to execute action [{}]: {:?}", &action_id, e);
                }
            }
        }.instrument(span));
        Ok(())
    }
}
