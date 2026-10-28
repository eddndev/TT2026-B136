// Spanish messages for the findings about time-stamp tokens and about the
// binary and text encodings inside the package. The codes are the stable
// codes of crates/evidence-verify. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.

const TOKEN_UNREADABLE = 'El sello de tiempo no tiene el formato que emite Qadra.';
const TOKEN_OTHER_CONTENT = 'El sello de tiempo no corresponde a este contenido.';
const TOKEN_SIGNATURE = 'La firma del sello de tiempo es incorrecta.';
const TOKEN_SIGNER = 'No se pudo identificar a la autoridad que firm\u00f3 el sello de tiempo.';

export const tokenMessages = {
  'token.verified': 'El sello de tiempo es correcto.',
  'token.anchor_pinned': 'La autoridad del sello de tiempo depende de una ra\u00edz fija en Qadra.',
  'token.anchor_not_pinned':
    'La autoridad del sello de tiempo no depende de una ra\u00edz de la lista fija de Qadra.',
  'token.missing': 'Falta el sello de tiempo.',
  'token.not_granted': 'La respuesta de la autoridad no incluye un sello de tiempo concedido.',
  'token.too_large': 'El sello de tiempo es m\u00e1s grande de lo permitido.',
  'token.malformed': TOKEN_UNREADABLE,
  'token.version': TOKEN_UNREADABLE,
  'token.certificate_choice': TOKEN_UNREADABLE,
  'token.critical_extension': TOKEN_UNREADABLE,
  'token.gen_time_fraction': TOKEN_UNREADABLE,
  'token.digest_algorithm': 'El sello de tiempo usa un m\u00e9todo de resumen que no se admite.',
  'token.ess_hash_algorithm': 'El sello de tiempo usa un m\u00e9todo de resumen que no se admite.',
  'token.signature_algorithm': 'El sello de tiempo usa un m\u00e9todo de firma que no se admite.',
  'token.policy_not_admitted':
    'El sello de tiempo se emiti\u00f3 con una pol\u00edtica que no se admite.',
  'token.imprint_mismatch': TOKEN_OTHER_CONTENT,
  'token.message_digest_mismatch': TOKEN_OTHER_CONTENT,
  'token.content_type_mismatch': TOKEN_OTHER_CONTENT,
  'token.ess_missing': 'Al sello de tiempo le falta la referencia a su certificado.',
  'token.ess_mismatch': 'El sello de tiempo hace referencia a otro certificado.',
  'token.signature_invalid': TOKEN_SIGNATURE,
  'token.signer_ambiguous': TOKEN_SIGNER,
  'token.signer_count': TOKEN_SIGNER,
  'token.signer_not_found': TOKEN_SIGNER,
  'token.signer_key': 'La autoridad del sello de tiempo usa un tipo de clave que no se admite.',
};

const ENCODING = 'Un archivo del paquete no tiene el formato esperado.';

export const encodingMessages = {
  ...Object.fromEntries(
    [
      'der.high_tag_number',
      'der.indefinite_length',
      'der.non_minimal_length',
      'der.too_deep',
      'der.too_many_children',
      'der.trailing_data',
      'der.truncated',
    ].map((code) => [code, ENCODING]),
  ),
  'pem.empty_block': 'Un archivo de certificado o de revocaci\u00f3n est\u00e1 vac\u00edo.',
  'pem.headers': ENCODING,
  'pem.invalid_base64': ENCODING,
  'pem.line_too_long': ENCODING,
  'pem.malformed_boundary': ENCODING,
  'pem.no_block': 'Un archivo de certificado o de revocaci\u00f3n no tiene contenido.',
  'pem.not_text': ENCODING,
  'pem.text_outside_block': ENCODING,
  'pem.too_many_blocks':
    'Un archivo de certificado o de revocaci\u00f3n tiene m\u00e1s contenido del esperado.',
  'pem.unterminated': ENCODING,
  'pem.wrong_label': 'Un archivo contiene otro tipo de dato del que indica su nombre.',
};

export const tokenFallbacks = {
  token: 'El sello de tiempo tiene un problema.',
  der: ENCODING,
  pem: ENCODING,
};
