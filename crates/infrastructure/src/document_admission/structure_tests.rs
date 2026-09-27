use super::structure::inspect;
use application::{
    documents::{AdmittedDocumentFormat as Format, DocumentUploadError},
    ApplicationError,
};

const PNG: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny.png");
const JPEG: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny.jpg");
const MP3: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny.mp3");
const WAV: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny.wav");
const MP4: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny.mp4");
const FAST_MP4: &[u8] = include_bytes!("../../tests/fixtures/media-admission/tiny-faststart.mp4");

fn rejected(bytes: &[u8]) {
    assert!(
        matches!(
            inspect(bytes),
            Err(ApplicationError::DocumentUpload(
                DocumentUploadError::Invalid | DocumentUploadError::Unsupported
            ))
        ),
        "malformed or unsupported structure was accepted"
    );
}
fn accepted(bytes: &[u8], format: Format) {
    assert_eq!(inspect(bytes).unwrap(), format);
}
fn marker(bytes: &[u8], value: &[u8]) -> usize {
    bytes
        .windows(value.len())
        .position(|part| part == value)
        .unwrap()
}
fn be32(bytes: &[u8], offset: usize) -> usize {
    u32::from_be_bytes(bytes[offset..offset + 4].try_into().unwrap()) as usize
}
fn put_be32(bytes: &mut [u8], offset: usize, value: usize) {
    bytes[offset..offset + 4].copy_from_slice(&(value as u32).to_be_bytes());
}
fn png_chunks(bytes: &[u8]) -> Vec<&[u8]> {
    let mut offset = 8;
    let mut chunks = Vec::new();
    while offset < bytes.len() {
        let end = offset + be32(bytes, offset) + 12;
        chunks.push(&bytes[offset..end]);
        offset = end;
    }
    chunks
}
fn png_from(chunks: &[&[u8]]) -> Vec<u8> {
    let mut bytes = PNG[..8].to_vec();
    for chunk in chunks {
        bytes.extend_from_slice(chunk);
    }
    bytes
}
fn box_bytes(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = ((payload.len() + 8) as u32).to_be_bytes().to_vec();
    bytes.extend_from_slice(kind);
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn admits_structures_of_supported_media_with_both_mp4_index_positions() {
    for (bytes, format) in [
        (PNG, Format::Png),
        (JPEG, Format::Jpeg),
        (MP3, Format::Mp3),
        (WAV, Format::Wav),
        (MP4, Format::Mp4),
        (FAST_MP4, Format::Mp4),
    ] {
        accepted(bytes, format);
    }
}

#[test]
fn png_rejects_late_crc_corruption_and_truncated_or_missing_iend() {
    let chunks = png_chunks(PNG);
    let idat = chunks
        .iter()
        .rposition(|chunk| &chunk[4..8] == b"IDAT")
        .unwrap();
    let mut damaged = chunks[idat].to_vec();
    let last = damaged.len() - 1;
    damaged[last] ^= 1;
    let mut replaced = chunks.clone();
    replaced[idat] = &damaged;
    rejected(&png_from(&replaced));
    assert_eq!(&chunks.last().unwrap()[4..8], b"IEND");
    rejected(&png_from(&chunks[..chunks.len() - 1]));
    rejected(&PNG[..PNG.len() - 1]);
}

#[test]
fn png_rejects_chunk_order_duplicate_header_and_bytes_after_iend() {
    let mut chunks = png_chunks(PNG);
    let ihdr = chunks[0];
    let idat = chunks
        .iter()
        .position(|chunk| &chunk[4..8] == b"IDAT")
        .unwrap();
    chunks.swap(0, idat);
    rejected(&png_from(&chunks));
    chunks = png_chunks(PNG);
    chunks.insert(1, ihdr);
    rejected(&png_from(&chunks));
    let mut trailing = PNG.to_vec();
    trailing.extend_from_slice(b"hidden payload");
    rejected(&trailing);
}

#[test]
fn jpeg_rejects_missing_eoi_invalid_segment_lengths_and_trailing_content() {
    assert_eq!(&JPEG[JPEG.len() - 2..], b"\xff\xd9");
    rejected(&JPEG[..JPEG.len() - 2]);
    rejected(b"\xff\xd8\xff\xe0\x00\x01\xff\xd9");
    rejected(b"\xff\xd8\xff\xe0\xff\xff\x00\xff\xd9");
    let mut trailing = JPEG.to_vec();
    trailing.extend_from_slice(b"hidden payload");
    rejected(&trailing);
    let mut concatenated = JPEG.to_vec();
    concatenated.extend_from_slice(JPEG);
    rejected(&concatenated);
}

// MPEG-1 layer III, 128 kbit/s, 44.1 kHz, no padding: 417 bytes per frame.
// These exercise framing only; decoder acceptance uses the independent fixtures.
fn mp3_frames() -> Vec<u8> {
    let mut frame = vec![0; 417];
    frame[..4].copy_from_slice(&[0xff, 0xfb, 0x90, 0]);
    [frame.clone(), frame].concat()
}

#[test]
fn mp3_requires_layer_three_and_complete_final_frame() {
    let valid = mp3_frames();
    accepted(&valid, Format::Mp3);
    rejected(&valid[..valid.len() - 1]);
    let mut layer_two = valid.clone();
    layer_two[1] = 0xfd;
    layer_two[418] = 0xfd;
    rejected(&layer_two);
    let mut tail = valid;
    tail.extend_from_slice(&[0xff, 0xfb, 0x90]);
    rejected(&tail);
}

#[test]
fn mp3_bounds_id3_sizes_and_rejects_non_synchsafe_tag_length() {
    let frames = mp3_frames();
    let mut bytes = b"ID3\x04\x00\x00\x00\x00\x00\x00".to_vec();
    bytes.extend_from_slice(&frames);
    accepted(&bytes, Format::Mp3);
    let mut oversized = bytes.clone();
    oversized[6..10].copy_from_slice(&[0x7f; 4]);
    rejected(&oversized);
    let mut invalid = bytes;
    invalid[9] = 0x80;
    rejected(&invalid);
    rejected(b"ID3\x04\x00\x00\x00");
}

fn wav_pcm(data: &[u8]) -> Vec<u8> {
    let mut bytes = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0".to_vec();
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&44100u32.to_le_bytes());
    bytes.extend_from_slice(&88200u32.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data.len() as u32).to_le_bytes());
    bytes.extend_from_slice(data);
    if !data.len().is_multiple_of(2) {
        bytes.push(0);
    }
    let size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    bytes
}

#[test]
fn wav_requires_exact_riff_and_pcm_chunk_lengths() {
    let valid = wav_pcm(&[0, 0, 1, 0]);
    accepted(&valid, Format::Wav);
    rejected(&valid[..valid.len() - 1]);
    let mut short = valid.clone();
    short[4..8].copy_from_slice(&1u32.to_le_bytes());
    rejected(&short);
    let mut data_overflow = valid.clone();
    data_overflow[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
    rejected(&data_overflow);
    let mut trailing = valid;
    trailing.push(0);
    rejected(&trailing);
}

#[test]
fn wav_requires_pcm_encoding_and_consistent_sample_alignment() {
    let valid = wav_pcm(&[0, 0, 1, 0]);
    rejected(&wav_pcm(&[0, 0, 1]));
    let mut alignment = valid.clone();
    alignment[32..34].copy_from_slice(&1u16.to_le_bytes());
    rejected(&alignment);
    let mut compressed = valid;
    compressed[20..22].copy_from_slice(&6u16.to_le_bytes());
    rejected(&compressed);
}

#[test]
fn mp4_requires_an_approved_brand_and_complete_top_level_boxes() {
    let mut bytes = MP4.to_vec();
    let ftyp = marker(&bytes, b"ftyp");
    let end = ftyp - 4 + be32(&bytes, ftyp - 4);
    bytes[ftyp + 4..ftyp + 8].copy_from_slice(b"qt  ");
    for brand in bytes[ftyp + 12..end].chunks_exact_mut(4) {
        brand.copy_from_slice(b"qt  ");
    }
    rejected(&bytes);
    rejected(&MP4[..MP4.len() - 1]);
    let mut overflow = MP4.to_vec();
    let moov = marker(&overflow, b"moov");
    put_be32(&mut overflow, moov - 4, u32::MAX as usize);
    rejected(&overflow);
}

#[test]
fn mp4_rejects_nested_boxes_that_escape_their_parent() {
    let mut bytes = MP4.to_vec();
    let trak = marker(&bytes, b"trak");
    let remaining = bytes.len() - (trak - 4);
    put_be32(&mut bytes, trak - 4, remaining + 1);
    rejected(&bytes);
}

#[test]
fn mp4_rejects_unbounded_container_nesting() {
    let mut nested = box_bytes(b"free", &[]);
    for _ in 0..64 {
        nested = box_bytes(b"trak", &nested);
    }
    let mut bytes = MP4.to_vec();
    let moov = marker(&bytes, b"moov") - 4;
    let size = be32(&bytes, moov);
    bytes.splice(moov + 8..moov + 8, nested.iter().copied());
    put_be32(&mut bytes, moov, size + nested.len());
    assert!(matches!(
        inspect(&bytes),
        Err(ApplicationError::DocumentUpload(
            DocumentUploadError::Invalid | DocumentUploadError::Limit
        ))
    ));
}

#[test]
fn mp4_rejects_external_data_reference_and_encrypted_sample_entry() {
    let mut external = MP4.to_vec();
    let dref = marker(&external, b"dref");
    assert!(be32(&external, dref + 8) > 0);
    assert_eq!(&external[dref + 16..dref + 20], b"url ");
    external[dref + 20..dref + 24].fill(0);
    rejected(&external);
    let mut encrypted = MP4.to_vec();
    let stsd = marker(&encrypted, b"stsd");
    assert!(be32(&encrypted, stsd + 8) > 0);
    assert_eq!(&encrypted[stsd + 16..stsd + 20], b"avc1");
    encrypted[stsd + 16..stsd + 20].copy_from_slice(b"encv");
    rejected(&encrypted);
}

fn change_chunk_offset(bytes: &mut [u8], offset: u64) {
    if let Some(tag) = bytes.windows(4).position(|part| part == b"stco") {
        assert!(be32(bytes, tag + 8) > 0);
        put_be32(bytes, tag + 12, offset as usize);
    } else {
        let tag = marker(bytes, b"co64");
        assert!(be32(bytes, tag + 8) > 0);
        bytes[tag + 12..tag + 20].copy_from_slice(&offset.to_be_bytes());
    }
}

#[test]
fn mp4_requires_chunk_offsets_inside_mdat_payload_not_just_inside_file() {
    for offset in [8, MP4.len() as u64 + 1] {
        let mut bytes = MP4.to_vec();
        change_chunk_offset(&mut bytes, offset);
        rejected(&bytes);
    }
}

#[test]
fn mp4_rejects_samples_whose_declared_bytes_escape_media_data() {
    let mut bytes = MP4.to_vec();
    let stsz = marker(&bytes, b"stsz");
    assert!(be32(&bytes, stsz + 12) > 0);
    put_be32(&mut bytes, stsz + 8, MP4.len());
    rejected(&bytes);
}

#[test]
fn txt_accepts_strict_utf8_with_optional_bom_and_explicit_whitespace() {
    accepted(b"Plain text\r\nwith\tcolumns\n", Format::Txt);
    accepted(b"\xef\xbb\xbfUTF-8: \xc3\xb1\n", Format::Txt);
}

#[test]
fn txt_rejects_invalid_utf8_nul_and_non_text_controls() {
    for bytes in [
        b"text\xc3".as_slice(),
        b"text\0tail".as_slice(),
        b"text\x01tail".as_slice(),
        b"text\x1btail".as_slice(),
        b"text\x7ftail".as_slice(),
        b"text\xc2\x85tail".as_slice(),
    ] {
        rejected(bytes);
    }
}

#[test]
fn known_gif_is_unsupported_instead_of_invalid_text() {
    for bytes in [b"GIF89a\x00".as_slice(), b"GIF87a\x01\x00"] {
        assert!(matches!(
            inspect(bytes),
            Err(ApplicationError::DocumentUpload(
                DocumentUploadError::Unsupported
            ))
        ));
    }
}
