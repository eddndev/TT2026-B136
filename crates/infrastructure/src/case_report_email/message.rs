use application::{case_reports::CaseReportEmailMessage, ApplicationError};
use reqwest::blocking::Body;
use serde::Serialize;
use std::io::Cursor;
use zeroize::Zeroizing;

#[derive(Serialize)]
struct Message<'a> {
    from: &'a str,
    to: [&'a str; 1],
    subject: &'a str,
    text: &'a str,
}

pub(super) fn valid(message: &CaseReportEmailMessage) -> bool {
    super::configuration::address(&message.from)
        && super::configuration::address(&message.to)
        && !message.subject.is_empty()
        && message.subject.len() <= 200
        && !message.subject.chars().any(char::is_control)
        && !message.text.is_empty()
        && message.text.len() <= 4096
}

pub(super) fn body(message: &CaseReportEmailMessage) -> Result<Body, ApplicationError> {
    let message = Message {
        from: &message.from,
        to: [&message.to],
        subject: &message.subject,
        text: &message.text,
    };
    let mut bytes = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *bytes, &message).map_err(|_| unavailable())?;
    let length = u64::try_from(bytes.len()).map_err(|_| unavailable())?;
    Ok(Body::sized(Cursor::new(bytes), length))
}

fn unavailable() -> ApplicationError {
    ApplicationError::Port("report email preparation failed".into())
}
