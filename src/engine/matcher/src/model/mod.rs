use std::collections::HashMap;
use tornado_common_api::Event;

/// The ProcessedEvent is the result of the matcher process.
/// It contains the original Event along with the result of the matching operation.
#[derive(Debug, Clone)]
pub struct ProcessedEvent<'o> {
    pub event: Event,
    pub matched: HashMap<&'o str, HashMap<&'o str, String>>,
}

impl<'o> ProcessedEvent<'o> {
    pub fn new(event: Event) -> ProcessedEvent<'o> {
        ProcessedEvent {
            event,
            matched: HashMap::new(),
        }
    }
}
