//! Startup verification for isolated support and upload parsers.

use crate::serve_args::ServeArgs;
use anyhow::Context;
use infrastructure::{
    document_admission::IsolatedDocumentUploadAdmission,
    document_formats::IsolatedDocumentFormatValidator,
};

pub fn open(
    args: &ServeArgs,
) -> anyhow::Result<(
    IsolatedDocumentFormatValidator,
    IsolatedDocumentUploadAdmission,
)> {
    let executable = std::env::current_exe().context("cannot locate document validation worker")?;
    let library =
        std::fs::canonicalize(&args.qpdf_library).context("cannot locate native qpdf library")?;
    let support = IsolatedDocumentFormatValidator::new(executable.clone(), library.clone())
        .context("cannot configure support format validation")?;
    support
        .check_configuration()
        .context("support format startup check failed")?;
    let admission = IsolatedDocumentUploadAdmission::new(
        executable,
        library,
        std::fs::canonicalize(&args.ffprobe_path).context("cannot locate multimedia probe")?,
        std::fs::canonicalize(&args.ffmpeg_path).context("cannot locate multimedia decoder")?,
    )
    .context("cannot configure document upload admission")?;
    admission
        .check_configuration()
        .context("document upload startup check failed")?;
    Ok((support, admission))
}
