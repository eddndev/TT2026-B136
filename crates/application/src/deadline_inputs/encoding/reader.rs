use super::invalid;
use crate::ApplicationError;
use domain::typed_participants::Uuid;

pub(crate) struct Reader<'a> {
    remaining: &'a [u8],
}
impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { remaining: bytes }
    }
    pub(crate) fn finished(&self) -> bool {
        self.remaining.is_empty()
    }
    pub(crate) fn take(&mut self, length: usize) -> Result<&'a [u8], ApplicationError> {
        if length > self.remaining.len() {
            return Err(invalid("truncated field"));
        }
        let (value, rest) = self.remaining.split_at(length);
        self.remaining = rest;
        Ok(value)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N], ApplicationError> {
        self.take(N)?.try_into().map_err(invalid)
    }
    pub(crate) fn byte(&mut self) -> Result<u8, ApplicationError> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn flag(&mut self) -> Result<bool, ApplicationError> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid("option or boolean tag is neither zero nor one")),
        }
    }
    pub(crate) fn uuid(&mut self) -> Result<Uuid, ApplicationError> {
        Ok(Uuid::from_bytes(self.array()?))
    }
    pub(crate) fn u16(&mut self) -> Result<u16, ApplicationError> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub(crate) fn u32(&mut self) -> Result<u32, ApplicationError> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub(crate) fn i32(&mut self) -> Result<i32, ApplicationError> {
        Ok(i32::from_be_bytes(self.array()?))
    }
    pub(crate) fn text(&mut self, max_bytes: usize) -> Result<&'a str, ApplicationError> {
        let length = usize::try_from(self.u32()?).map_err(invalid)?;
        if length > max_bytes {
            return Err(invalid("text exceeds its byte limit"));
        }
        std::str::from_utf8(self.take(length)?).map_err(invalid)
    }
}
