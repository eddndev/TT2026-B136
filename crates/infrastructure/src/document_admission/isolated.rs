use application::{
    documents::{
        AdmittedDocumentFormat as Format, DocumentUploadAdmission, DocumentUploadError,
        MAX_DOCUMENT_UPLOAD_BYTES,
    },
    ApplicationError,
};
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Validates immutable upload bytes in bounded, sequential native processes.
pub struct IsolatedDocumentUploadAdmission {
    executable: PathBuf,
    library: PathBuf,
    probe: PathBuf,
    decoder: PathBuf,
}
impl IsolatedDocumentUploadAdmission {
    pub fn new(
        executable: PathBuf,
        library: PathBuf,
        probe: PathBuf,
        decoder: PathBuf,
    ) -> Result<Self, ApplicationError> {
        for path in [&executable, &library, &probe, &decoder] {
            if !path.is_absolute() || !path.is_file() {
                return Err(DocumentUploadError::Unavailable.into());
            }
        }
        Ok(Self {
            executable,
            library,
            probe,
            decoder,
        })
    }
    /// Checks every supported family against the configured decoder build.
    pub fn check_configuration(&self) -> Result<(), ApplicationError> {
        let inputs: [&[u8]; 8] = [
            include_bytes!("../../tests/fixtures/stage-support.pdf"),
            include_bytes!("../document_formats/docx/tests/fixtures/producer.docx"),
            b"text\n",
            include_bytes!("../../tests/fixtures/media-admission/tiny.jpg"),
            include_bytes!("../../tests/fixtures/media-admission/tiny.png"),
            include_bytes!("../../tests/fixtures/media-admission/tiny.mp3"),
            include_bytes!("../../tests/fixtures/media-admission/tiny.wav"),
            include_bytes!("../../tests/fixtures/media-admission/tiny.mp4"),
        ];
        for bytes in inputs {
            self.validate(bytes)?;
        }
        Ok(())
    }
}
impl DocumentUploadAdmission for IsolatedDocumentUploadAdmission {
    fn validate(&self, bytes: &[u8]) -> Result<Format, ApplicationError> {
        if bytes.len() > MAX_DOCUMENT_UPLOAD_BYTES {
            return Err(DocumentUploadError::Limit.into());
        }
        #[cfg(target_os = "linux")]
        {
            use rustix::fs::{
                fcntl_add_seals, fcntl_get_seals, memfd_create, MemfdFlags, SealFlags,
            };
            use std::{
                fs::File,
                io::{Seek, Write},
            };
            let unavailable = |_| DocumentUploadError::Unavailable;
            let deadline = Instant::now() + Duration::from_secs(20);
            let fd = memfd_create(
                "document-admission",
                MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
            )
            .map_err(unavailable)?;
            let mut input = File::from(fd);
            input
                .write_all(bytes)
                .map_err(|_| DocumentUploadError::Unavailable)?;
            let seals = SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL;
            fcntl_add_seals(&input, seals).map_err(unavailable)?;
            if fcntl_get_seals(&input).map_err(unavailable)? != seals {
                return Err(DocumentUploadError::Unavailable.into());
            }
            input
                .rewind()
                .map_err(|_| DocumentUploadError::Unavailable)?;
            let result = super::process::run(
                &self.executable,
                &[
                    super::worker::INSPECT_FLAG.into(),
                    self.library.as_os_str().to_owned(),
                ],
                &input,
                deadline.min(Instant::now() + Duration::from_secs(10)),
                7,
            )?;
            let format = super::worker::decode(&result)?;
            if matches!(format, Format::Pdf | Format::Docx | Format::Txt) {
                return Ok(format);
            }
            for (program, probe) in [(&self.probe, true), (&self.decoder, false)] {
                input
                    .rewind()
                    .map_err(|_| DocumentUploadError::Unavailable)?;
                let args = super::media::arguments(format, Path::new(program), probe);
                let result = super::process::run(
                    &self.executable,
                    &args,
                    &input,
                    deadline,
                    if probe { 65536 } else { 0 },
                )?;
                if probe {
                    super::media::inspect_probe(format, &result)?;
                }
            }
            Ok(format)
        }
        #[cfg(not(target_os = "linux"))]
        Err(DocumentUploadError::Unavailable.into())
    }
}
