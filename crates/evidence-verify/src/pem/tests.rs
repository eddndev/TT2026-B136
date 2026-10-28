use super::{decode_all, decode_one, PemError};

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut group = [0u8; 3];
        group[..chunk.len()].copy_from_slice(chunk);
        let value = (u32::from(group[0]) << 16) | (u32::from(group[1]) << 8) | u32::from(group[2]);
        for index in 0..4 {
            if index <= chunk.len() {
                let symbol = (value >> (18 - 6 * index)) & 0x3f;
                out.push(char::from(ALPHABET[symbol as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The same layout as the server writer: 64 columns, LF line ends.
fn server_pem(label: &str, bytes: &[u8]) -> String {
    let encoded = encode(bytes);
    let mut pem = format!("-----BEGIN {label}-----\n");
    for line in encoded.as_bytes().chunks(64) {
        pem.push_str(core::str::from_utf8(line).unwrap());
        pem.push('\n');
    }
    pem.push_str(&format!("-----END {label}-----\n"));
    pem
}

fn sample(length: usize) -> Vec<u8> {
    (0..length).map(|index| (index * 7 + 3) as u8).collect()
}

const CERT: &str = "CERTIFICATE";

fn one(text: &str) -> Result<Vec<u8>, PemError> {
    decode_one(text.as_bytes(), CERT)
}

#[test]
fn decodes_server_output_for_every_padding_and_line_shape() {
    for length in [1, 2, 3, 47, 48, 49, 95, 96, 97, 1000] {
        let bytes = sample(length);
        let pem = server_pem(CERT, &bytes);
        assert_eq!(one(&pem), Ok(bytes), "length {length}");
    }
}

#[test]
fn decodes_crlf_line_ends() {
    let bytes = sample(200);
    let pem = server_pem(CERT, &bytes).replace('\n', "\r\n");
    assert_eq!(one(&pem), Ok(bytes));
}

#[test]
fn decodes_mixed_lf_and_crlf_line_ends() {
    let bytes = sample(200);
    let pem = server_pem(CERT, &bytes).replacen('\n', "\r\n", 2);
    assert_eq!(one(&pem), Ok(bytes));
}

#[test]
fn decodes_without_a_final_newline() {
    let bytes = sample(100);
    let pem = server_pem(CERT, &bytes);
    assert_eq!(one(pem.trim_end_matches('\n')), Ok(bytes));
}

#[test]
fn decodes_lines_shorter_than_64_columns() {
    let bytes = sample(90);
    let encoded = encode(&bytes);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for line in encoded.as_bytes().chunks(20) {
        pem.push_str(core::str::from_utf8(line).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    assert_eq!(one(&pem), Ok(bytes));
}

#[test]
fn admits_whitespace_only_lines_around_blocks() {
    let bytes = sample(10);
    let pem = format!("\n \t\r\n{}\n  \n\t", server_pem(CERT, &bytes));
    assert_eq!(one(&pem), Ok(bytes));
}

#[test]
fn decodes_a_chain_in_order() {
    let first = sample(70);
    let second = sample(130);
    let pem = server_pem(CERT, &first) + &server_pem(CERT, &second);
    let blocks = decode_all(pem.as_bytes(), CERT, 4);
    assert_eq!(blocks, Ok(vec![first, second]));
}

#[test]
fn admits_exactly_max_blocks() {
    let pem = server_pem(CERT, &sample(5)).repeat(3);
    assert_eq!(decode_all(pem.as_bytes(), CERT, 3).map(|b| b.len()), Ok(3));
}

#[test]
fn rejects_more_blocks_than_admitted() {
    let pem = server_pem(CERT, &sample(5)).repeat(3);
    assert_eq!(
        decode_all(pem.as_bytes(), CERT, 2),
        Err(PemError::TooManyBlocks)
    );
    assert_eq!(one(&pem), Err(PemError::TooManyBlocks));
    let single = server_pem(CERT, &sample(5));
    assert_eq!(
        decode_all(single.as_bytes(), CERT, 0),
        Err(PemError::TooManyBlocks)
    );
}

#[test]
fn rejects_input_without_blocks() {
    for text in ["", "\n", " \r\n\t\n"] {
        assert_eq!(one(text), Err(PemError::NoBlock), "{text:?}");
    }
}

#[test]
fn rejects_another_label() {
    let pem = server_pem("X509 CRL", &sample(20));
    assert_eq!(one(&pem), Err(PemError::WrongLabel));
    let lower = server_pem("certificate", &sample(20));
    assert_eq!(one(&lower), Err(PemError::WrongLabel));
}

#[test]
fn rejects_an_end_boundary_with_another_label() {
    let pem = server_pem(CERT, &sample(20)).replace("END CERTIFICATE", "END X509 CRL");
    assert_eq!(one(&pem), Err(PemError::WrongLabel));
}

#[test]
fn rejects_a_chain_with_one_foreign_block() {
    let pem = server_pem(CERT, &sample(20)) + &server_pem("PRIVATE KEY", &sample(20));
    assert_eq!(
        decode_all(pem.as_bytes(), CERT, 4),
        Err(PemError::WrongLabel)
    );
}

#[test]
fn rejects_text_outside_blocks() {
    let pem = server_pem(CERT, &sample(20));
    let cases = [
        format!("Subject: CN=x\n{pem}"),
        format!("{pem}trailing\n"),
        format!("{pem}x"),
        format!(" {pem}"),
    ];
    for case in cases {
        assert_eq!(one(&case), Err(PemError::TextOutsideBlock), "{case:?}");
    }
}

#[test]
fn rejects_malformed_boundaries() {
    let pem = server_pem(CERT, &sample(20));
    let cases = [
        pem.replace("-----BEGIN CERTIFICATE-----", "-----BEGIN CERTIFICATE----"),
        pem.replace(
            "-----BEGIN CERTIFICATE-----",
            "-----BEGIN CERTIFICATE----- ",
        ),
        pem.replace("-----END CERTIFICATE-----", "-----END CERTIFICATE----- "),
        pem.replace("-----END CERTIFICATE-----", "-----END CERTIFICATE"),
        pem.replace("-----END CERTIFICATE-----", "-----END-----"),
        format!("-----END CERTIFICATE-----\n{pem}"),
        pem.replacen('\n', "\n-----BEGIN CERTIFICATE-----\n", 1),
    ];
    for case in cases {
        assert_eq!(one(&case), Err(PemError::MalformedBoundary), "{case:?}");
    }
}

#[test]
fn rejects_encapsulated_headers() {
    let body = encode(&sample(20));
    let pem = format!(
        "-----BEGIN CERTIFICATE-----\nProc-Type: 4,ENCRYPTED\n\
         DEK-Info: AES-128-CBC,00\n\n{body}\n-----END CERTIFICATE-----\n"
    );
    assert_eq!(one(&pem), Err(PemError::Headers));
}

#[test]
fn rejects_blank_or_padded_lines_inside_a_block() {
    let pem = server_pem(CERT, &sample(100));
    let cases = [
        pem.replacen('\n', "\n\n", 1),
        pem.replacen('\n', "\n \n", 1),
        pem.replacen('\n', "\n ", 1),
        pem.replacen("\n-----END", " \n-----END", 1),
    ];
    for case in cases {
        assert_eq!(one(&case), Err(PemError::InvalidBase64), "{case:?}");
    }
}

#[test]
fn rejects_lines_longer_than_64_columns() {
    let bytes = sample(60);
    let encoded = encode(&bytes);
    assert_eq!(encoded.len(), 80);
    let pem = format!("-----BEGIN CERTIFICATE-----\n{encoded}\n-----END CERTIFICATE-----\n");
    assert_eq!(one(&pem), Err(PemError::LineTooLong));
}

#[test]
fn rejects_invalid_base64() {
    let pem = server_pem(CERT, &sample(20));
    let body = encode(&sample(20));
    let cases = [
        pem.replacen(&body[..4], "Zm9-", 1),
        pem.replacen(&body[..4], "Zm9_", 1),
        pem.replace(&body, &body[..body.len() - 1]),
        pem.replace(&body, &format!("{}\nZm9v", &body)),
        "-----BEGIN CERTIFICATE-----\nZh==\n-----END CERTIFICATE-----\n".to_string(),
    ];
    for case in cases {
        assert_eq!(one(&case), Err(PemError::InvalidBase64), "{case:?}");
    }
}

#[test]
fn rejects_empty_blocks() {
    let pem = "-----BEGIN CERTIFICATE-----\n-----END CERTIFICATE-----\n";
    assert_eq!(one(pem), Err(PemError::EmptyBlock));
}

#[test]
fn rejects_an_unterminated_block() {
    let pem = server_pem(CERT, &sample(20));
    let cut = &pem[..pem.find("-----END").unwrap()];
    assert_eq!(one(cut), Err(PemError::Unterminated));
    assert_eq!(
        one("-----BEGIN CERTIFICATE-----"),
        Err(PemError::Unterminated)
    );
}

#[test]
fn rejects_bytes_that_are_not_text() {
    let pem = server_pem(CERT, &sample(20));
    let mut cases = vec![
        pem.replacen('\n', "\r", 1),
        pem.replace("-----\n", "-----\r"),
        format!("{pem}\r"),
        format!("{pem}\u{0}"),
        format!("\u{1b}\n{pem}"),
        format!("{pem}\u{e9}\n"),
    ]
    .into_iter()
    .map(String::into_bytes)
    .collect::<Vec<_>>();
    let mut invalid_utf8 = pem.clone().into_bytes();
    invalid_utf8.extend_from_slice(&[0xff, b'\n']);
    cases.push(invalid_utf8);
    for case in cases {
        assert_eq!(decode_one(&case, CERT), Err(PemError::NotText), "{case:?}");
    }
}

#[test]
fn error_codes_are_stable_and_distinct() {
    let all = [
        (PemError::NotText, "pem.not_text"),
        (PemError::TextOutsideBlock, "pem.text_outside_block"),
        (PemError::WrongLabel, "pem.wrong_label"),
        (PemError::MalformedBoundary, "pem.malformed_boundary"),
        (PemError::Headers, "pem.headers"),
        (PemError::LineTooLong, "pem.line_too_long"),
        (PemError::InvalidBase64, "pem.invalid_base64"),
        (PemError::EmptyBlock, "pem.empty_block"),
        (PemError::Unterminated, "pem.unterminated"),
        (PemError::NoBlock, "pem.no_block"),
        (PemError::TooManyBlocks, "pem.too_many_blocks"),
    ];
    for (error, code) in all {
        assert_eq!(error.code(), code);
        assert_eq!(error.to_string(), code);
    }
}
