use crate::metrics::{
    ActionMeter, ACTION_ID_LABEL_KEY, ATTEMPT_RESULT_KEY, RESULT_FAILURE, RESULT_SUCCESS,
};
use std::borrow::Cow;
use std::cell::RefCell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;
use tornado_common_api::TracedAction;
use tornado_executor_common::{ExecutorError, StatefulExecutor, StatelessExecutor};

pub mod callback;
pub mod pool;
pub mod retry;

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait Command<Message, Output> {
    async fn execute(&self, message: Message) -> Output;
}

pub struct StatelessExecutorCommand<T: StatelessExecutor> {
    action_meter: Arc<ActionMeter>,
    executor: T,
}

impl<T: StatelessExecutor> StatelessExecutorCommand<T> {
    pub fn new(action_meter: Arc<ActionMeter>, executor: T) -> Self {
        Self { action_meter, executor }
    }
}

/// Implement the Command pattern for StatelessExecutorCommand
#[async_trait::async_trait(?Send)]
impl<T: StatelessExecutor> Command<TracedAction, Result<(), ExecutorError>>
    for StatelessExecutorCommand<T>
{
    async fn execute(&self, message: TracedAction) -> Result<(), ExecutorError> {
        let action_id = message.action.id.to_owned();
        let result = self.executor.execute(message).await;
        increment_processing_attempt_counter(&result, action_id, self.action_meter.as_ref());
        result
    }
}

#[inline]
fn increment_processing_attempt_counter<T: Into<Cow<'static, str>>>(
    result: &Result<(), ExecutorError>,
    action_id: T,
    action_meter: &ActionMeter,
) {
    let action_id_label = ACTION_ID_LABEL_KEY.string(action_id);
    match result {
        Ok(_) => action_meter
            .actions_processing_attempts_counter
            .add(1, &[action_id_label, ATTEMPT_RESULT_KEY.string(RESULT_SUCCESS)]),
        Err(_) => action_meter
            .actions_processing_attempts_counter
            .add(1, &[action_id_label, ATTEMPT_RESULT_KEY.string(RESULT_FAILURE)]),
    };
}

/// Basic Trait to implement the Command Design Pattern.
/// See: https://refactoring.guru/design-patterns/command
#[async_trait::async_trait(?Send)]
pub trait CommandMut<Message, Output> {
    async fn execute(&mut self, message: Message) -> Output;
}

pub struct StatefulExecutorCommand<T: StatefulExecutor> {
    action_meter: Arc<ActionMeter>,
    executor: T,
}

impl<T: StatefulExecutor> StatefulExecutorCommand<T> {
    pub fn new(action_meter: Arc<ActionMeter>, executor: T) -> Self {
        Self { action_meter, executor }
    }
}

/// Implement the Command pattern for StatefulExecutorCommand
#[async_trait::async_trait(?Send)]
impl<T: StatefulExecutor> CommandMut<TracedAction, Result<(), ExecutorError>>
    for StatefulExecutorCommand<T>
{
    async fn execute(&mut self, message: TracedAction) -> Result<(), ExecutorError> {
        let action_id = message.action.id.to_owned();
        let result = self.executor.execute(message).await;
        increment_processing_attempt_counter(&result, action_id, self.action_meter.as_ref());
        result
    }
}

pub struct CommandMutWrapper<Message, Output, C: CommandMut<Message, Output>> {
    command: Rc<RefCell<C>>,
    phantom_message: PhantomData<Message>,
    phantom_output: PhantomData<Output>,
}

impl<Message, Output, C: CommandMut<Message, Output>> CommandMutWrapper<Message, Output, C> {
    pub fn new(command: Rc<RefCell<C>>) -> Self {
        Self { command, phantom_message: PhantomData, phantom_output: PhantomData }
    }
}

#[async_trait::async_trait(?Send)]
impl<I, O, T: CommandMut<I, O>> Command<I, O> for CommandMutWrapper<I, O, T> {
    async fn execute(&self, message: I) -> O {
        let mut command = self.command.borrow_mut();
        command.execute(message).await
    }
}
