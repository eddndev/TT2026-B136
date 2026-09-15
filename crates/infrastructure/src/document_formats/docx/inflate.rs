use super::{budget::DocxBudget, rejected, zip_layout::Entry, Error, Result};
use crc32fast::Hasher;
use flate2::{Decompress, FlushDecompress, Status};

/// Drains each entry once, charging actual output and checking the exact stream end.
pub(super) fn read_part(bytes: &[u8], entry: &Entry, budget: &mut DocxBudget) -> Result<Vec<u8>> {
    let compressed = bytes
        .get(entry.data..entry.data + entry.compressed)
        .ok_or(Error::StageSupportFormatRejected)?;
    let mut result = Vec::new();
    let mut crc = Hasher::new();
    if entry.method == 0 {
        for chunk in compressed.chunks(8192) {
            append(chunk, budget, &mut crc, &mut result)?;
        }
    } else {
        let mut decoder = Decompress::new(false);
        let mut buffer = [0u8; 8192];
        loop {
            let input_before = decoder.total_in();
            let output_before = decoder.total_out();
            let status = decoder
                .decompress(
                    &compressed[input_before as usize..],
                    &mut buffer,
                    FlushDecompress::None,
                )
                .map_err(|_| Error::StageSupportFormatRejected)?;
            let count = (decoder.total_out() - output_before) as usize;
            append(&buffer[..count], budget, &mut crc, &mut result)?;
            if status == Status::StreamEnd {
                if decoder.total_in() != compressed.len() as u64 {
                    return rejected();
                }
                break;
            }
            if decoder.total_in() == input_before && count == 0 {
                return rejected();
            }
        }
    }
    if result.len() != entry.size || crc.finalize() != entry.crc {
        return rejected();
    }
    Ok(result)
}
fn append(
    bytes: &[u8],
    budget: &mut DocxBudget,
    crc: &mut Hasher,
    result: &mut Vec<u8>,
) -> Result<()> {
    budget.bytes(bytes.len())?;
    crc.update(bytes);
    result.extend_from_slice(bytes);
    Ok(())
}
