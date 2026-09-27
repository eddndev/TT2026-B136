use super::media::inspect_probe;
use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadError},
    ApplicationError,
};
use serde_json::{json, Value};

fn video(codec: &str) -> Value {
    json!({"index":0,"codec_name":codec,"codec_type":"video","width":32,"height":32,
        "disposition":{"attached_pic":0}})
}
fn audio(codec: &str) -> Value {
    json!({"index":1,"codec_name":codec,"codec_type":"audio","sample_rate":"8000","channels":1,
        "disposition":{"attached_pic":0}})
}
fn probe(format: Format, streams: Vec<Value>) -> Result<(), ApplicationError> {
    inspect_probe(
        format,
        &serde_json::to_vec(&json!({"streams":streams})).unwrap(),
    )
}
#[test]
fn accepts_only_the_declared_family_and_supported_streams() {
    for (format, streams) in [
        (Format::Jpeg, vec![video("mjpeg")]),
        (Format::Png, vec![video("png")]),
        (Format::Mp3, vec![audio("mp3")]),
        (Format::Wav, vec![audio("pcm_s16le")]),
        (Format::Mp4, vec![video("h264"), audio("aac")]),
    ] {
        probe(format, streams).unwrap();
    }
}
#[test]
fn rejects_unknown_codecs_extra_tracks_and_incomplete_probe_data() {
    for streams in [
        vec![],
        vec![video("hevc")],
        vec![video("h264"), audio("opus")],
        vec![json!({"index":0,"codec_name":"h264","codec_type":"video"})],
        vec![
            video("h264"),
            json!({"index":1,"codec_type":"data","codec_name":"bin_data"}),
        ],
    ] {
        assert!(probe(Format::Mp4, streams).is_err());
    }
    assert!(probe(Format::Png, vec![video("png"), audio("aac")]).is_err());
    let mut attached = video("h264");
    attached["disposition"]["attached_pic"] = json!(1);
    assert!(probe(Format::Mp4, vec![attached]).is_err());
    assert!(inspect_probe(Format::Mp4, br#"{"streams":[],"error":{"code":-1}}"#).is_err());
    assert!(inspect_probe(Format::Mp4, b"{}").is_err());
    assert!(inspect_probe(Format::Mp4, b"not json").is_err());
}
#[test]
fn rejects_probe_resource_overflow_and_duplicate_stream_indices() {
    let mut big = video("h264");
    big["width"] = json!(8192);
    big["height"] = json!(8192);
    assert!(matches!(
        probe(Format::Mp4, vec![big]),
        Err(ApplicationError::DocumentUpload(DocumentUploadError::Limit))
    ));
    for (field, value) in [("channels", json!(9)), ("sample_rate", json!("192001"))] {
        let mut stream = audio("aac");
        stream[field] = value;
        assert!(matches!(
            probe(Format::Mp4, vec![stream]),
            Err(ApplicationError::DocumentUpload(DocumentUploadError::Limit))
        ));
    }
    assert!(probe(Format::Mp4, vec![video("h264"), video("h264")]).is_err());
    assert!(probe(Format::Mp4, vec![audio("aac"); 9]).is_err());
}
