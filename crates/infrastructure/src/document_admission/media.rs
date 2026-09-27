use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadError},
    ApplicationError,
};
use serde_json::Value;
use std::{collections::BTreeSet, ffi::OsString, path::Path};

fn invalid() -> ApplicationError {
    DocumentUploadError::Invalid.into()
}
fn unsupported() -> ApplicationError {
    DocumentUploadError::Unsupported.into()
}
fn limit() -> ApplicationError {
    DocumentUploadError::Limit.into()
}

pub(super) fn inspect_probe(format: Format, bytes: &[u8]) -> Result<(), ApplicationError> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if value.get("error").is_some() {
        return Err(invalid());
    }
    let streams = value
        .get("streams")
        .and_then(Value::as_array)
        .ok_or_else(invalid)?;
    if streams.is_empty() {
        return Err(invalid());
    }
    if streams.len() > 8 {
        return Err(limit());
    }
    if format != Format::Mp4 && streams.len() != 1 {
        return Err(unsupported());
    }
    let mut indices = BTreeSet::new();
    for stream in streams {
        let index = stream
            .get("index")
            .and_then(Value::as_u64)
            .ok_or_else(invalid)?;
        if !indices.insert(index) {
            return Err(invalid());
        }
        if stream
            .pointer("/disposition/attached_pic")
            .and_then(Value::as_u64)
            != Some(0)
        {
            return Err(unsupported());
        }
        let codec = stream
            .get("codec_name")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let kind = stream
            .get("codec_type")
            .and_then(Value::as_str)
            .ok_or_else(invalid)?;
        let admitted = match (format, kind) {
            (Format::Jpeg, "video") => codec == "mjpeg",
            (Format::Png, "video") => codec == "png",
            (Format::Mp3, "audio") => codec == "mp3",
            (Format::Wav, "audio") => {
                matches!(codec, "pcm_u8" | "pcm_s16le" | "pcm_s24le" | "pcm_s32le")
            }
            (Format::Mp4, "video") => codec == "h264",
            (Format::Mp4, "audio") => codec == "aac",
            _ => false,
        };
        if !admitted {
            return Err(unsupported());
        }
        if kind == "video" {
            let width = stream
                .get("width")
                .and_then(Value::as_u64)
                .ok_or_else(invalid)?;
            let height = stream
                .get("height")
                .and_then(Value::as_u64)
                .ok_or_else(invalid)?;
            if width == 0 || height == 0 {
                return Err(invalid());
            }
            if width
                .checked_mul(height)
                .is_none_or(|pixels| pixels > 16_777_216)
            {
                return Err(limit());
            }
        } else {
            let channels = stream
                .get("channels")
                .and_then(Value::as_u64)
                .ok_or_else(invalid)?;
            let rate = stream
                .get("sample_rate")
                .and_then(Value::as_str)
                .and_then(|rate| rate.parse::<u64>().ok())
                .ok_or_else(invalid)?;
            if channels == 0 || rate == 0 {
                return Err(invalid());
            }
            if channels > 8 || rate > 192_000 {
                return Err(limit());
            }
        }
    }
    Ok(())
}

/// Private worker arguments contain only administrative paths and fixed options.
pub(super) fn arguments(format: Format, program: &Path, probe: bool) -> Vec<OsString> {
    let family = match format {
        Format::Jpeg => "jpeg_pipe",
        Format::Png => "png_pipe",
        Format::Mp3 => "mp3",
        Format::Wav => "wav",
        Format::Mp4 => "mov",
        _ => unreachable!("non-media input"),
    };
    let mut values: Vec<OsString> = vec![
        super::worker::EXEC_FLAG.into(),
        (if probe { "probe" } else { "decode" }).into(),
        program.as_os_str().to_owned(),
    ];
    let mut push = |options: &[&str]| values.extend(options.iter().map(OsString::from));
    if probe {
        push(&["-v", "error"]);
    } else {
        push(&[
            "-hide_banner",
            "-nostdin",
            "-nostats",
            "-loglevel",
            "error",
            "-xerror",
            "-max_error_rate",
            "0",
            "-abort_on",
            "empty_output+empty_output_stream",
            "-filter_threads",
            "1",
            "-filter_complex_threads",
            "1",
            "-err_detect",
            "crccheck+bitstream+buffer+explode+careful",
        ]);
    }
    if format == Format::Mp4 {
        push(&[
            "-enable_drefs",
            "0",
            "-use_absolute_path",
            "0",
            "-ignore_editlist",
            "1",
        ]);
    }
    push(&[
        "-protocol_whitelist",
        "fd",
        "-format_whitelist",
        family,
        "-f",
        family,
        "-codec_whitelist",
        "h264,aac,mp3,mp3float,mjpeg,png,pcm_u8,pcm_s16le,pcm_s24le,pcm_s32le",
        "-threads",
        "1",
        "-max_pixels",
        "16777216",
        "-max_streams",
        "8",
        "-fd",
        "0",
        "-i",
        "fd:",
    ]);
    if probe {
        push(&["-show_error","-show_entries",
            "stream=index,codec_name,codec_type,codec_tag_string,width,height,sample_rate,channels:stream_disposition=attached_pic","-of","json"]);
    } else {
        push(&[
            "-map",
            "0",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-threads",
            "1",
            "-f",
            "null",
            "-",
        ]);
    }
    values
}
