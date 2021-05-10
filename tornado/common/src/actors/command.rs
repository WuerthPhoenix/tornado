use std::rc::Rc;
use crate::command::Command;
use actix::{Actor, Context, Handler};
use crate::actors::message::ActionMessage;
use tornado_executor_common::ExecutorError;
use log::*;
use tornado_common_api::Action;

pub struct CommandExecutorActor<T: Command<Rc<Action>, Result<(), ExecutorError>> + 'static> {
    pub command: Rc<T>,
}

impl <T: Command<Rc<Action>, Result<(), ExecutorError>> + 'static> CommandExecutorActor<T> {

    pub fn new(command: Rc<T>) -> Self {
        Self {
            command,
        }
    }

}

impl <T: Command<Rc<Action>, Result<(), ExecutorError>> + 'static> Actor for CommandExecutorActor<T> {
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("CommandExecutorActor started.");
    }
}

impl <T: Command<Rc<Action>, Result<(), ExecutorError>> + 'static> Handler<ActionMessage> for CommandExecutorActor<T> {
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut Context<Self>) -> Self::Result {
        trace!("CommandExecutorActor - received new action [{:?}]", &msg.action);

        let action = msg.action.clone();
        let command = self.command.clone();

        actix::spawn(async move {
            match command.execute(action).await {
                Ok(_) => {
                    debug!("CommandExecutorActor - Action executed successfully");
                }
                Err(e) => {
                    error!(
                        "CommandExecutorActor - Failed to execute action: {:?}", e
                    );
                }
            }
        });
        Ok(())
    }
}