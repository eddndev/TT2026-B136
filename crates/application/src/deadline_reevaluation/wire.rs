use super::{DependencyFamily, TrackingCodecError};
use domain::{cases::CaseId, crypto::Sha256Digest, typed_participants::Uuid};

type Result<T> = std::result::Result<T, TrackingCodecError>;

pub(super) struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}
impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8], prefix: &[u8; 5], limit: usize) -> Result<Self> {
        if bytes.len() > limit {
            return Err(TrackingCodecError::SizeLimit);
        }
        if !bytes.starts_with(prefix) {
            return Err(TrackingCodecError::InvalidEncoding("prefix"));
        }
        Ok(Self { bytes, offset: 5 })
    }
    fn take(&mut self, length: usize) -> Result<&'a [u8]> {
        let end = self
            .offset
            .checked_add(length)
            .ok_or(TrackingCodecError::InvalidEncoding("length"))?;
        let value = self
            .bytes
            .get(self.offset..end)
            .ok_or(TrackingCodecError::InvalidEncoding("truncated"))?;
        self.offset = end;
        Ok(value)
    }
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        self.take(N)?
            .try_into()
            .map_err(|_| TrackingCodecError::InvalidEncoding("array"))
    }
    pub(super) fn byte(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    pub(super) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_be_bytes(self.array()?))
    }
    pub(super) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_be_bytes(self.array()?))
    }
    pub(super) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_be_bytes(self.array()?))
    }
    pub(super) fn uuid(&mut self) -> Result<Uuid> {
        Ok(Uuid::from_bytes(self.array()?))
    }
    pub(super) fn case_id(&mut self) -> Result<CaseId> {
        Ok(CaseId::from_uuid(self.uuid()?))
    }
    pub(super) fn digest(&mut self) -> Result<Sha256Digest> {
        Ok(Sha256Digest::from_array(self.array()?))
    }
    pub(super) fn optional<T>(
        &mut self,
        read: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<Option<T>> {
        match self.byte()? {
            0 => Ok(None),
            1 => read(self).map(Some),
            _ => Err(TrackingCodecError::InvalidEncoding("presence tag")),
        }
    }
    pub(super) fn text(&mut self, scalar_limit: usize) -> Result<String> {
        let length = usize::try_from(self.u64()?)
            .map_err(|_| TrackingCodecError::InvalidEncoding("text length"))?;
        if length > scalar_limit * 4 {
            return Err(TrackingCodecError::SizeLimit);
        }
        let value = std::str::from_utf8(self.take(length)?)
            .map_err(|_| TrackingCodecError::InvalidEncoding("UTF-8"))?;
        if value.chars().count() > scalar_limit {
            return Err(TrackingCodecError::SizeLimit);
        }
        Ok(value.to_owned())
    }
    pub(super) fn family(&mut self) -> Result<DependencyFamily> {
        match self.byte()? {
            0 => Ok(DependencyFamily::Resolution),
            1 => Ok(DependencyFamily::Notification),
            2 => Ok(DependencyFamily::HearingResult),
            3 => Ok(DependencyFamily::Calendar),
            4 => Ok(DependencyFamily::Profile),
            _ => Err(TrackingCodecError::InvalidEncoding("dependency family")),
        }
    }
    pub(super) fn finish(self) -> Result<()> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(TrackingCodecError::InvalidEncoding("trailing bytes"))
        }
    }
}

pub(super) fn text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}
pub(super) fn optional<T>(
    bytes: &mut Vec<u8>,
    value: Option<T>,
    write: impl FnOnce(&mut Vec<u8>, T),
) {
    if let Some(value) = value {
        bytes.push(1);
        write(bytes, value);
    } else {
        bytes.push(0);
    }
}
pub(super) fn case_id(bytes: &mut Vec<u8>, value: CaseId) {
    uuid(bytes, value.as_uuid());
}
pub(super) fn uuid(bytes: &mut Vec<u8>, value: Uuid) {
    bytes.extend_from_slice(value.as_bytes());
}
