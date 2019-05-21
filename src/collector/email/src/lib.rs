use log::warn;
use mailparse::{dateparse, parse_mail, DispositionType, MailHeaderMap, MailParseError};
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Number, Value};

/// The Email Collector receives a MIME email message as input, parses it and produces a Tornado Event.
#[derive(Default)]
pub struct EmailEventCollector {}

impl EmailEventCollector {
    pub fn new() -> EmailEventCollector {
        Default::default()
    }
}

impl<'a> Collector<&'a [u8]> for EmailEventCollector {
    fn to_event(&self, input: &'a [u8]) -> Result<Event, CollectorError> {
        let email = parse_mail(input).map_err(into_err)?;

        let subject = email
            .headers
            .get_first_value("Subject")
            .map_err(into_err)?
            .unwrap_or_else(|| "".to_owned());
        let date = dateparse(
            email
                .headers
                .get_first_value("Date")
                .map_err(into_err)?
                .unwrap_or_else(|| "".to_owned())
                .as_str(),
        )
        .map_err(|err| CollectorError::EventCreationError { message: err.to_string() })?;

        let mut bodies = vec![];

        for subpart in email.subparts {
            let content_disposition = subpart.get_content_disposition().map_err(into_err)?;
            match content_disposition.disposition {
                DispositionType::Inline => {
                    if subpart.ctype.mimetype.contains("text") {
                        bodies.push(Value::Text(subpart.get_body().map_err(into_err)?))
                    }
                }
                DispositionType::Attachment => {}
                _ => {
                    warn!(
                        "Ignore email subpart with DispositionType: {:?}",
                        content_disposition.disposition
                    );
                }
            }
        }
        //let body = email.subparts[0].get_body().map_err(into_err)?;

        let mut event = Event::new("email");
        event.payload.insert("date".to_owned(), Value::Number(Number::PosInt(date as u64)));
        event.payload.insert("subject".to_owned(), Value::Text(subject));
        event.payload.insert("body".to_owned(), Value::Array(bodies));

        Ok(event)
    }
}

fn into_err(err: MailParseError) -> CollectorError {
    CollectorError::EventCreationError { message: format!("{}", err) }
}

#[cfg(test)]
mod test {

    use super::*;
    use std::fs;

    #[test]
    fn should_produce_event_from_email_input() {
        // Arrange
        let email = get_email("./test_resources/email_01_input.txt");
        let mut expected_event = get_event("./test_resources/email_01_output.json");

        let collector = EmailEventCollector::new();

        // Act
        let event = collector.to_event(email.as_bytes()).unwrap();

        // Assert
        expected_event.created_ms = event.created_ms.clone();
        assert_eq!(expected_event, event);
    }

    fn get_email(path: &str) -> String {
        fs::read_to_string(path).expect(&format!("Unable to open the file [{}]", path))
    }

    fn get_event(path: &str) -> Event {
        let event_string =
            fs::read_to_string(path).expect(&format!("Unable to open the file [{}]", path));
        serde_json::from_str(&event_string)
            .expect(&format!("Cannot parse event from file [{}]", path))
    }
}
