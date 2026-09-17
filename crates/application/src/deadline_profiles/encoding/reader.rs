use super::{invalid, DeadlineProfileError};
use crate::deadline_inputs::encoding::{read, reader::Reader as InputReader};
use domain::{
    deadline_arithmetic::ArithmeticRule,
    deadline_triggers::TriggerRequirement,
    judicial_calendars::CivilDate,
    procedural_facts::{FactLabel, FactText},
    procedural_time::DeclaredProceduralTime,
    typed_participants::Uuid,
};
use std::num::NonZeroU32;

pub(super) struct Reader<'a> {
    inner: InputReader<'a>,
}
impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
        Self {
            inner: InputReader::new(bytes),
        }
    }
    pub(super) fn finished(&self) -> bool {
        self.inner.finished()
    }
    pub(super) fn take(&mut self, length: usize) -> Result<&'a [u8], DeadlineProfileError> {
        self.inner.take(length).map_err(invalid)
    }
    pub(super) fn byte(&mut self) -> Result<u8, DeadlineProfileError> {
        self.inner.byte().map_err(invalid)
    }
    pub(super) fn flag(&mut self) -> Result<bool, DeadlineProfileError> {
        self.inner.flag().map_err(invalid)
    }
    pub(super) fn uuid(&mut self) -> Result<Uuid, DeadlineProfileError> {
        self.inner.uuid().map_err(invalid)
    }
    pub(super) fn u32(&mut self) -> Result<u32, DeadlineProfileError> {
        self.inner.u32().map_err(invalid)
    }
    pub(super) fn i32(&mut self) -> Result<i32, DeadlineProfileError> {
        self.inner.i32().map_err(invalid)
    }
    pub(super) fn i64(&mut self) -> Result<i64, DeadlineProfileError> {
        Ok(i64::from_be_bytes(
            self.take(8)?.try_into().map_err(invalid)?,
        ))
    }
    pub(super) fn text(&mut self, max_bytes: usize) -> Result<&'a str, DeadlineProfileError> {
        self.inner.text(max_bytes).map_err(invalid)
    }
    pub(super) fn label(&mut self) -> Result<FactLabel, DeadlineProfileError> {
        FactLabel::new(self.text(800)?).map_err(invalid)
    }
    pub(super) fn fact_text(&mut self) -> Result<FactText, DeadlineProfileError> {
        FactText::new(self.text(4000)?).map_err(invalid)
    }
    pub(super) fn date(&mut self) -> Result<CivilDate, DeadlineProfileError> {
        CivilDate::from_days_since_epoch(self.i32()?).map_err(invalid)
    }
    pub(super) fn declared_time(&mut self) -> Result<DeclaredProceduralTime, DeadlineProfileError> {
        read::declared_time(&mut self.inner).map_err(invalid)
    }
    pub(super) fn requirement(&mut self) -> Result<TriggerRequirement, DeadlineProfileError> {
        read::requirement(&mut self.inner).map_err(invalid)
    }
    pub(super) fn rule(&mut self) -> Result<ArithmeticRule, DeadlineProfileError> {
        read::rule(&mut self.inner).map_err(invalid)
    }
    pub(super) fn quantity(&mut self) -> Result<NonZeroU32, DeadlineProfileError> {
        NonZeroU32::new(self.u32()?).ok_or_else(|| invalid("zero quantity"))
    }
    pub(super) fn optional_quantity(&mut self) -> Result<Option<NonZeroU32>, DeadlineProfileError> {
        if self.flag()? {
            Ok(Some(self.quantity()?))
        } else {
            Ok(None)
        }
    }
    pub(super) fn count(&mut self) -> Result<usize, DeadlineProfileError> {
        let count = self.u32()?;
        if !(1..=16).contains(&count) {
            return Err(invalid("collection count"));
        }
        usize::try_from(count).map_err(invalid)
    }
    pub(super) fn reference_ids(&mut self) -> Result<Vec<Uuid>, DeadlineProfileError> {
        let count = self.count()?;
        let mut ids = Vec::with_capacity(count);
        for _ in 0..count {
            ids.push(self.uuid()?);
        }
        Ok(ids)
    }
}
