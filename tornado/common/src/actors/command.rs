use crate::actors::message::ActionMessage;
use crate::command::Command;
use actix::{Actor, Addr, Context, Handler};
use log::*;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common_api::Action;
use tornado_executor_common::ExecutorError;
use tracing_futures::Instrument;
use tornado_common_api::metrics::{ACTION_ID_LABEL_KEY, ACTION_RESULT_KEY, ACTION_RESULT_SUCCESS, ACTION_RESULT_FAILURE, ActionMeter};

pub struct CommandExecutorActor<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> {
    pub command: Rc<T>,
    action_meter: Arc<ActionMeter>
}

impl<T: Command<Arc<Action>, Result<(), ExecutorError>> + 'static> CommandExecutorActor<T> {
    pub fn start_new(
        message_mailbox_capacity: usize,
        command: Rc<T>,
        action_meter: Arc<ActionMeter>
    ) -> Addr<CommandExecutorActor<T>> {
        CommandExecutorActor::create(move |ctx| {
            ctx.set_mailbox_capacity(message_mailbox_capacity);
            Self { command, action_meter }
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
        let command = self.command.clone();
        let action_meter = self.action_meter.clone();

        let action = msg.action;
        actix::spawn(
            async move {
                let action_id = action.id.clone();
                trace!("CommandExecutorActor - received new action [{:?}]", &action);
                debug!("CommandExecutorActor - Execute action [{:?}]", &action_id);

                let action_id_label = ACTION_ID_LABEL_KEY.string(action.id.to_owned());

                match command.execute(action).await {
                    Ok(_) => {
                        action_meter.actions_processed_counter.add(1, &[
                            action_id_label,
                            ACTION_RESULT_KEY.string(ACTION_RESULT_SUCCESS)
                        ]);
                        debug!(
                            "CommandExecutorActor - Action [{}] executed successfully",
                            &action_id
                        );
                    }
                    Err(e) => {
                        action_meter.actions_processed_counter.add(1, &[
                            action_id_label,
                            ACTION_RESULT_KEY.string(ACTION_RESULT_FAILURE)
                        ]);
                        error!(
                            "CommandExecutorActor - Failed to execute action [{}]: {:?}",
                            &action_id, e
                        );
                    }
                }
            }
            .instrument(msg.span),
        );
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::command::retry::test::{AlwaysFailExecutor, AlwaysOkExecutor};
    use tokio::sync::mpsc::unbounded_channel;
    use tornado_common_metrics::prometheus::{TextEncoder, Encoder};
    use tornado_common_metrics::opentelemetry::sdk::Resource;
    use tornado_common_metrics::opentelemetry::KeyValue;

    #[actix_rt::test]
    async fn should_increase_counter_if_action_succeeds() {
        // Arrange
        let prometheus_exporter = opentelemetry_prometheus::exporter()
            .with_resource(Resource::new(vec![KeyValue::new("app", "test_app")]))
            .init();
        let (sender, mut receiver) = unbounded_channel();

        let action = Arc::new(Action::new("trace_id", "id"));
        let span = tracing::Span::current();
        let message = ActionMessage { action, span };
        let action_meter = Arc::new(ActionMeter::new("test_action_meter"));
        let executor = CommandExecutorActor::start_new(
            10,
            Rc::new(AlwaysOkExecutor { sender: sender.clone() }),
            action_meter.clone()
        );

        // Act
        executor.try_send(message).unwrap();
        let _received = receiver.recv().await.unwrap();

        // Assert
        let encoder = TextEncoder::new();
        let metric_families = prometheus_exporter.registry().gather();
        let mut buf = Vec::new();
        encoder.encode(&metric_families, &mut buf).unwrap();
        let result = std::str::from_utf8(&buf).unwrap();
        assert!(result.contains("actions_processed_counter{action_id=\"id\",action_result=\"success\",app=\"test_app\"} 1"));
    }

    #[actix_rt::test]
    async fn should_increase_counter_if_action_fails() {
        // Arrange
        let prometheus_exporter = opentelemetry_prometheus::exporter()
            .with_resource(Resource::new(vec![KeyValue::new("app", "test_app")]))
            .init();
        let (sender, mut receiver) = unbounded_channel();

        let action = Arc::new(Action::new("trace_id", "id"));
        let span = tracing::Span::current();
        let message = ActionMessage { action, span };
        let action_meter = Arc::new(ActionMeter::new("test_action_meter"));
        let executor = CommandExecutorActor::start_new(
            10,
            Rc::new(AlwaysFailExecutor { sender: sender.clone(), can_retry: true }),
            action_meter.clone()
        );

        // Act
        executor.try_send(message).unwrap();
        let _received = receiver.recv().await.unwrap();

        // Assert
        let encoder = TextEncoder::new();
        let metric_families = prometheus_exporter.registry().gather();
        let mut buf = Vec::new();
        encoder.encode(&metric_families, &mut buf).unwrap();
        let result = std::str::from_utf8(&buf).unwrap();
        assert!(result.contains("actions_processed_counter{action_id=\"id\",action_result=\"failure\",app=\"test_app\"} 1"));
    }
}
