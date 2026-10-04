use crate::procedural_time::{DeclaredProceduralPrecision as Precision, DeclaredProceduralTime};

use super::{MeasureTime, MeasureValidity};

impl MeasureValidity {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut bytes = b"MVAL1".to_vec();
        measure_time(&mut bytes, self.start());
        text(&mut bytes, self.statement().as_str());
        bytes.push(u8::from(self.end().is_some()));
        if let Some(end) = self.end() {
            measure_time(&mut bytes, end);
        }
        bytes
    }
}

fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u32).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn measure_time(bytes: &mut Vec<u8>, value: &MeasureTime) {
    declared_time(bytes, value.declared());
    bytes.push(u8::from(value.unknown_reason().is_some()));
    if let Some(reason) = value.unknown_reason() {
        text(bytes, reason.as_str());
    }
}

fn declared_time(bytes: &mut Vec<u8>, value: DeclaredProceduralTime) {
    bytes.push(match value.precision() {
        Precision::Unknown => 0,
        Precision::Date => 1,
        Precision::Minute => 2,
        Precision::Second => 3,
    });
    if let Some(date) = value.local_date() {
        let date = date.date();
        bytes.extend_from_slice(&(date.year() as u16).to_be_bytes());
        bytes.push(date.month() as u8);
        bytes.push(date.day());
        if let Some(hour) = value.local_hour() {
            bytes.push(hour);
        }
        if let Some(minute) = value.local_minute() {
            bytes.push(minute);
        }
        if let Some(second) = value.local_second() {
            bytes.push(second);
        }
        bytes.push(u8::from(value.offset().is_some()));
        if let Some(offset) = value.offset() {
            bytes.extend_from_slice(&offset.whole_seconds().to_be_bytes());
        }
    }
}
