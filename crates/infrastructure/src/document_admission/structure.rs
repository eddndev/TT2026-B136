use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadError, MAX_DOCUMENT_UPLOAD_BYTES},
    ApplicationError,
};

#[path = "structure_audio.rs"]
mod audio;
#[path = "structure_images.rs"]
mod images;
#[path = "structure_mp4.rs"]
mod mp4;

type Result<T> = std::result::Result<T, ApplicationError>;
fn invalid() -> ApplicationError {
    DocumentUploadError::Invalid.into()
}
fn unsupported() -> ApplicationError {
    DocumentUploadError::Unsupported.into()
}
fn limit() -> ApplicationError {
    DocumentUploadError::Limit.into()
}
fn require(value: bool) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(invalid())
    }
}
fn be16(bytes: &[u8], at: usize) -> Result<u16> {
    let value = bytes
        .get(at..at.checked_add(2).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    Ok(u16::from_be_bytes(value.try_into().map_err(|_| invalid())?))
}
fn be32(bytes: &[u8], at: usize) -> Result<u32> {
    let value = bytes
        .get(at..at.checked_add(4).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    Ok(u32::from_be_bytes(value.try_into().map_err(|_| invalid())?))
}
fn pixels(width: u32, height: u32) -> Result<()> {
    require(width != 0 && height != 0)?;
    if u64::from(width) * u64::from(height) > 16_777_216 {
        return Err(limit());
    }
    Ok(())
}
fn audio_limits(channels: u32, rate: u32) -> Result<()> {
    require(channels != 0 && rate != 0)?;
    if channels > 8 || rate > 192_000 {
        return Err(limit());
    }
    Ok(())
}

/// Structural admission only. PDF/DOCX still require their native/XML worker
/// checks, and multimedia must pass complete isolated decoding afterwards.
pub(super) fn inspect(bytes: &[u8]) -> Result<Format> {
    if bytes.len() > MAX_DOCUMENT_UPLOAD_BYTES {
        return Err(limit());
    }
    require(!bytes.is_empty())?;
    if bytes.starts_with(b"GIF89a") || bytes.starts_with(b"GIF87a") {
        return Err(unsupported());
    }
    if bytes.starts_with(b"%PDF-") {
        return Ok(Format::Pdf);
    }
    if bytes.starts_with(b"PK\x03\x04") {
        return Ok(Format::Docx);
    }
    if bytes.starts_with(b"\x89PNG") {
        images::png(bytes)?;
        return Ok(Format::Png);
    }
    if bytes.starts_with(b"\xff\xd8") {
        images::jpeg(bytes)?;
        return Ok(Format::Jpeg);
    }
    if bytes.starts_with(b"RIFF") {
        audio::wav(bytes)?;
        return Ok(Format::Wav);
    }
    if bytes.starts_with(b"RF64") || bytes.starts_with(b"RIFX") {
        return Err(unsupported());
    }
    if bytes.get(4..8) == Some(b"ftyp") {
        mp4::inspect(bytes)?;
        return Ok(Format::Mp4);
    }
    if bytes.starts_with(b"ID3")
        || bytes
            .get(..2)
            .is_some_and(|v| v[0] == 0xff && v[1] & 0xe0 == 0xe0)
    {
        audio::mp3(bytes)?;
        return Ok(Format::Mp3);
    }
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| unsupported())?;
    require(!text.is_empty())?;
    require(
        !text
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\t' | '\r' | '\n')),
    )?;
    Ok(Format::Txt)
}
