use log::*;
use mailparse::body::Body;
use mailparse::{
    dateparse, parse_mail, DispositionType, MailHeaderMap, MailParseError, ParsedMail,
};
use tornado_collector_common::{Collector, CollectorError};
use tornado_common_api::{Event, Number, Payload, Value};

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
        trace!("EmailEventCollector - Received a new email");
        let email = parse_mail(input).map_err(into_err)?;

        trace!("EmailEventCollector - Parsed email: \n {:?}", email);

        let subject = get_first_header_value_or_empty(&email, "Subject")?;
        let from = get_first_header_value_or_empty(&email, "From")?;
        let to = get_first_header_value_or_empty(&email, "To")?;
        let cc = get_first_header_value_or_empty(&email, "Cc")?;

        let date = dateparse(get_first_header_value_or_empty(&email, "Date")?.as_str())
            .map_err(|err| CollectorError::EventCreationError { message: err.to_string() })?;

        let mut body = None;
        let mut attachments = vec![];

        extract_body_and_attachments(&email, &mut body, &mut attachments)?;

        for subpart in email.subparts {
            extract_body_and_attachments(&subpart, &mut body, &mut attachments)?;
        }

        let mut event = Event::new("email");
        event.payload.insert("date".to_owned(), Value::Number(Number::PosInt(date as u64)));
        event.payload.insert("subject".to_owned(), Value::Text(subject));
        event.payload.insert("from".to_owned(), Value::Text(from));
        event.payload.insert("to".to_owned(), Value::Text(to));
        event.payload.insert("cc".to_owned(), Value::Text(cc));
        event.payload.insert("body".to_owned(), body.unwrap_or_else(|| Value::Text("".to_owned())));
        event.payload.insert("attachments".to_owned(), Value::Array(attachments));

        Ok(event)
    }
}

fn into_err(err: MailParseError) -> CollectorError {
    CollectorError::EventCreationError { message: format!("{}", err) }
}

fn get_first_header_value_or_empty(
    email: &ParsedMail,
    header: &str,
) -> Result<String, CollectorError> {
    Ok(email.headers.get_first_value(header).unwrap_or_else(|| "".to_owned()))
}

fn extract_body_and_attachments(
    email: &ParsedMail,
    body: &mut Option<Value>,
    attachments: &mut Vec<Value>,
) -> Result<(), CollectorError> {
    let content_disposition = email.get_content_disposition();
    match content_disposition.disposition {
        DispositionType::Inline => {
            if email.ctype.mimetype.contains("text") {
                *body = match body.take() {
                    None => Some(Value::Text(email.get_body().map_err(into_err)?)),
                    opt => {
                        warn!("Found more than one body. Only the first one will be used.");
                        opt
                    }
                };
            } else {
                warn!("Found inline attachment not of type text. It will be ignored.");
            }
        }
        DispositionType::Attachment => {
            let mut attachment = Payload::new();
            attachment.insert(
                "filename".to_owned(),
                Value::Text(
                    content_disposition
                        .params
                        .get("filename")
                        .map(std::borrow::ToOwned::to_owned)
                        .unwrap_or_else(|| "".to_owned()),
                ),
            );
            attachment.insert("mime_type".to_owned(), Value::Text(email.ctype.mimetype.clone()));

            if email.ctype.mimetype.contains("text") {
                attachment.insert("encoding".to_owned(), Value::Text("plaintext".to_owned()));
                attachment
                    .insert("content".to_owned(), Value::Text(email.get_body().map_err(into_err)?));
            } else {
                attachment.insert("encoding".to_owned(), Value::Text("base64".to_owned()));

                let base64_content = match &email.get_body_encoded() {
                    Body::Base64(body) | Body::QuotedPrintable(body) => String::from_utf8(
                        body.get_raw()
                            .iter()
                            .filter(|c| !c.is_ascii_whitespace())
                            .cloned()
                            .collect(),
                    )
                    .map_err(|err| CollectorError::EventCreationError {
                        message: format!("{}", err),
                    })?,
                    Body::SevenBit(body) | Body::EightBit(body) => base64::encode(body.get_raw()),
                    Body::Binary(body) => base64::encode(body.get_raw()),
                };

                attachment.insert("content".to_owned(), Value::Text(base64_content));
            }
            attachments.push(Value::Map(attachment))
        }
        _ => {
            warn!(
                "Ignore email subpart with DispositionType: {:?}",
                content_disposition.disposition
            );
        }
    };
    Ok(())
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
        expected_event.trace_id = event.trace_id.clone();
        expected_event.created_ms = event.created_ms.clone();
        assert_eq!(expected_event, event);
    }

    #[test]
    fn should_should_parse_sender_and_recipients() {
        // Arrange
        let email = get_email("./test_resources/email_02_input.txt");
        let collector = EmailEventCollector::new();

        // Act
        let event = collector.to_event(email.as_bytes()).unwrap();

        // Assert
        assert_eq!(
            r#""Mr.Francesco.Cina" <mr.francesco.cina@gmail.com>"#,
            event.payload.get("from").unwrap()
        );
        assert_eq!(
            r#""Groeber, Benjamin" <Benjamin.Groeber@wuerth-phoenix.com>, francesco cina <mr.francesco.cina@gmail.com>"#,
            event.payload.get("to").unwrap()
        );
        assert_eq!(
            r#"Thomas.Forrer@wuerth-phoenix.com, mr.francesco.cina@gmail.com"#,
            event.payload.get("cc").unwrap()
        );
    }

    #[test]
    fn should_should_parse_body_and_subject() {
        // Arrange
        let email = get_email("./test_resources/email_02_input.txt");
        let collector = EmailEventCollector::new();

        // Act
        let event = collector.to_event(email.as_bytes()).unwrap();

        // Assert
        assert_eq!("Test for Mail collector", event.payload.get("subject").unwrap());
        assert!(event
            .payload
            .get("body")
            .unwrap()
            .get_text()
            .unwrap()
            .contains("<b>Test for Mail collector</b>"));
    }

    #[test]
    fn should_should_parse_text_attachments() {
        // Arrange
        let email = get_email("./test_resources/email_03_input.txt");
        let collector = EmailEventCollector::new();

        // Act
        let event = collector.to_event(email.as_bytes()).unwrap();

        // Assert
        assert_eq!(
            "Test for Mail collector - with attachments",
            event.payload.get("subject").unwrap()
        );
        assert!(event
            .payload
            .get("body")
            .unwrap()
            .get_text()
            .unwrap()
            .contains("<b>Test for Mail collector with attachments</b>"));

        let attachments = event.payload.get("attachments").unwrap().get_array().unwrap();
        assert_eq!(2, attachments.len());

        let attachment_0 = attachments[0].get_map().unwrap();
        assert_eq!("sample.pdf", attachment_0.get("filename").unwrap());
        assert_eq!("application/pdf", attachment_0.get("mime_type").unwrap());
        assert_eq!("base64", attachment_0.get("encoding").unwrap());
        assert!(attachment_0.get("content").unwrap().get_text().unwrap().starts_with(
            "JVBERi0xLjMNCiXi48/TDQoNCjEgMCBvYmoNCjw8DQovVHlwZSAvQ2F0YWxvZw0KL091dGxp"
        ));
        assert!(attachment_0.get("content").unwrap().get_text().unwrap().ends_with("T0YNCg=="));

        let attachment_1 = attachments[1].get_map().unwrap();
        assert_eq!("sample.txt", attachment_1.get("filename").unwrap());
        assert_eq!("text/plain", attachment_1.get("mime_type").unwrap());
        assert_eq!("plaintext", attachment_1.get("encoding").unwrap());
        assert_eq!(
            "txt file context for email collector\n1234567890987654321\n",
            attachment_1.get("content").unwrap()
        );
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
