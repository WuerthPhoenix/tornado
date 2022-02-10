use crate::actors::message::ActionMessage;
use crate::command::Command;
use crate::metrics::{
    ActionMeter, ACTION_ID_LABEL_KEY, ACTION_RESULT_KEY, RESULT_FAILURE, RESULT_SUCCESS,
};
use actix::{Actor, Addr, Context, Handler};
use log::*;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common_api::TracedAction;
use tornado_executor_common::ExecutorError;
use tracing_futures::Instrument;

pub struct CommandExecutorActor<T: Command<TracedAction, Result<(), ExecutorError>> + 'static> {
    pub command: Rc<T>,
    action_meter: Arc<ActionMeter>,
}

impl<T: Command<TracedAction, Result<(), ExecutorError>> + 'static> CommandExecutorActor<T> {
    pub fn start_new(
        message_mailbox_capacity: usize,
        command: Rc<T>,
        action_meter: Arc<ActionMeter>,
    ) -> Addr<CommandExecutorActor<T>> {
        CommandExecutorActor::create(move |ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            Self { command, action_meter }
        })
    }
}

impl<T: Command<TracedAction, Result<(), ExecutorError>> + 'static> Actor
    for CommandExecutorActor<T>
{
    type Context = Context<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("CommandExecutorActor started.");
    }
}

impl<T: Command<TracedAction, Result<(), ExecutorError>> + 'static> Handler<ActionMessage>
    for CommandExecutorActor<T>
{
    type Result = Result<(), ExecutorError>;

    fn handle(&mut self, msg: ActionMessage, _: &mut Context<Self>) -> Self::Result {
        let command = self.command.clone();
        let action_meter = self.action_meter.clone();

        let msg_to_executor = msg.0.to_owned();
        let action_id = msg.0.action.id.to_owned();
        actix::spawn(
            async move {
                trace!(
                    "CommandExecutorActor - received new action [{:?}]",
                    &msg_to_executor.action
                );
                debug!("CommandExecutorActor - Execute action [{:?}]", &action_id);

                let action_id_label = ACTION_ID_LABEL_KEY.string(action_id.to_owned());

                match command.execute(msg_to_executor).await {
                    Ok(_) => {
                        action_meter
                            .actions_processed_counter
                            .add(1, &[action_id_label, ACTION_RESULT_KEY.string(RESULT_SUCCESS)]);
                        debug!(
                            "CommandExecutorActor - Action [{}] executed successfully",
                            &action_id
                        );
                    }
                    Err(e) => {
                        action_meter
                            .actions_processed_counter
                            .add(1, &[action_id_label, ACTION_RESULT_KEY.string(RESULT_FAILURE)]);
                        error!(
                            "CommandExecutorActor - Failed to execute action [{}]: {:?}",
                            &action_id, e
                        );
                    }
                }
            }
            .instrument(msg.0.span.clone()),
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::retry::test::{AlwaysFailExecutor, AlwaysOkExecutor};
    use crate::command::StatelessExecutorCommand;
    use crate::root_test::prometheus_exporter;
    use tokio::sync::mpsc::unbounded_channel;
    use tokio::time::Duration;
    use tornado_common_api::TracedAction;
    use tornado_common_metrics::prometheus::{Encoder, TextEncoder};

    #[actix_rt::test]
    async fn should_increase_processed_and_attempts_counters_if_action_succeeds() {
        // Arrange
        let prometheus_exporter = prometheus_exporter();
        let (sender, mut receiver) = unbounded_channel();

        let action_id = format!("{}", rand::random::<usize>());
        let action = Arc::new(Action::new(action_id.clone()));
        let span = tracing::Span::current();
        let message = ActionMessage(TracedAction { action, span });
        let action_meter = Arc::new(ActionMeter::new("test_action_meter"));

        let stateless_executor_command = StatelessExecutorCommand::new(
            action_meter.clone(),
            AlwaysOkExecutor { sender: sender.clone() },
        );
        let executor = CommandExecutorActor::start_new(
            10,
            Rc::new(stateless_executor_command),
            action_meter.clone(),
        );

        // Act
        executor.try_send(message).unwrap();
        let _received = receiver.recv().await.unwrap();

        // Assert
        let mut result = "";
        let mut buf = Vec::new();
        while result.is_empty() {
            let encoder = TextEncoder::new();
            let metric_families = prometheus_exporter.registry().gather();
            encoder.encode(&metric_families, &mut buf).unwrap();
            result = std::str::from_utf8(&buf).unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(result.contains(&format!("action_id=\"{}\",action_result=\"success\"", action_id)));
        assert!(result.contains(&format!(
            "action_id=\"{}\",app=\"test_app\",attempt_result=\"success\"",
            action_id
        )));
    }

    #[actix_rt::test]
    async fn should_increase_processed_and_attempts_counters_if_action_fails() {
        // Arrange
        let prometheus_exporter = prometheus_exporter();
        let (sender, mut receiver) = unbounded_channel();

        let action_id = format!("{}", rand::random::<usize>());
        let action = Arc::new(Action::new(action_id.clone()));
        let span = tracing::Span::current();
        let message = ActionMessage(TracedAction { action, span });
        let action_meter = Arc::new(ActionMeter::new("test_action_meter"));
        let stateless_executor_command = StatelessExecutorCommand::new(
            action_meter.clone(),
            AlwaysFailExecutor { sender: sender.clone(), can_retry: true },
        );
        let executor = CommandExecutorActor::start_new(
            10,
            Rc::new(stateless_executor_command),
            action_meter.clone(),
        );

        // Act
        executor.try_send(message).unwrap();
        let _received = receiver.recv().await.unwrap();

        // Assert
        let encoder = TextEncoder::new();
        let metric_families = prometheus_exporter.registry().gather();
        let mut result = "";
        let mut buf = Vec::new();
        while result.is_empty() {
            encoder.encode(&metric_families, &mut buf).unwrap();
            result = std::str::from_utf8(&buf).unwrap();
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert!(result.contains(&format!("action_id=\"{}\",action_result=\"failure\"", action_id)));
        assert!(result.contains(&format!(
            "action_id=\"{}\",app=\"test_app\",attempt_result=\"failure\"",
            action_id
        )));
    }
}
