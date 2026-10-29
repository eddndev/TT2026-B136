// Spanish messages for the findings about the document signature, the
// binding and consent statements of individual signing and the consistency
// of dates and records. The codes are the stable codes of
// crates/evidence-verify. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.

export const signatureMessages = {
  'signature.valid': 'La firma del documento es correcta.',
  'signature.invalid': 'La firma del documento es incorrecta.',
  'signature.key_unavailable':
    'No se pudo leer el certificado de quien firm\u00f3, as\u00ed que la firma no se pudo revisar.',
  'binding.signature_valid': 'La firma que vincula el certificado con la cuenta es correcta.',
  'binding.signature_invalid': 'La firma que vincula el certificado con la cuenta es incorrecta.',
  'binding.certificate_mismatch':
    'La vinculaci\u00f3n de la cuenta se refiere a otro certificado distinto del incluido.',
  'binding.root_mismatch':
    'La vinculaci\u00f3n de la cuenta se refiere a otra ra\u00edz de confianza.',
  'consent.signature_valid': 'La firma del consentimiento es correcta.',
  'consent.signature_invalid': 'La firma del consentimiento es incorrecta.',
  'consent.matches_package': 'El consentimiento corresponde a este documento y a su firma.',
  'consent.certificate_mismatch': 'El consentimiento se refiere a otro certificado.',
  'consent.document_mismatch': 'El consentimiento se refiere a otro documento.',
  'consent.root_mismatch': 'El consentimiento se refiere a otra ra\u00edz de confianza.',
  'consent.signature_mismatch': 'El consentimiento se refiere a otra firma del documento.',
};

const STATEMENT_UNREADABLE =
  'Una declaraci\u00f3n firmada del paquete no tiene el formato esperado.';

export const statementMessages = {
  'statement.account_version':
    'Una declaraci\u00f3n firmada tiene un dato de la cuenta no v\u00e1lido.',
  'statement.document_version':
    'Una declaraci\u00f3n firmada tiene un n\u00famero de versi\u00f3n no v\u00e1lido.',
  'statement.format': STATEMENT_UNREADABLE,
  'statement.length': STATEMENT_UNREADABLE,
  'statement.nil_identity': 'Una declaraci\u00f3n firmada tiene un identificador vac\u00edo.',
  'statement.not_canonical': STATEMENT_UNREADABLE,
  'statement.purpose': 'Una declaraci\u00f3n firmada no es del tipo esperado.',
  'statement.trust_revision':
    'Una declaraci\u00f3n firmada tiene una revisi\u00f3n de confianza no v\u00e1lida.',
  'statements.same_binding': 'La vinculaci\u00f3n y el consentimiento describen la misma cuenta.',
  'statements.binding_mismatch':
    'La vinculaci\u00f3n y el consentimiento describen cuentas distintas.',
};

export const contextMessages = {
  'context.token_time': 'La fecha del sello de tiempo del documento qued\u00f3 establecida.',
  'context.token_time_unknown': 'No se pudo establecer la fecha de un sello de tiempo del paquete.',
  'context.records_not_time_stamped':
    'Alguna fecha viene de un registro de Qadra que ning\u00fan sello de tiempo respalda.',
  'context.records_unavailable': 'No se pudieron leer las fechas de registro y aceptaci\u00f3n.',
  'context.registration_before_acceptance':
    'El certificado se registr\u00f3 antes de aceptar la firma, como corresponde.',
  'context.registration_after_acceptance':
    'La fecha de registro del certificado es posterior a la aceptaci\u00f3n de la firma.',
  'context.token_before_acceptance':
    'Los sellos de tiempo del paquete son anteriores a la aceptaci\u00f3n, como corresponde.',
  'context.token_after_acceptance':
    'El sello de tiempo del documento es posterior a la aceptaci\u00f3n de la firma.',
  'context.same_root': 'La ra\u00edz del paquete es la misma que la de la vinculaci\u00f3n.',
  'context.root_mismatch': 'El paquete incluye dos ra\u00edces de confianza distintas.',
  'context.root_unavailable': 'No se pudo leer la ra\u00edz de confianza del paquete.',
  'context.same_trust':
    'Las listas de revocaci\u00f3n del registro y de la aceptaci\u00f3n coinciden.',
  'context.trust_mismatch':
    'Las listas de revocaci\u00f3n del registro y de la aceptaci\u00f3n no coinciden.',
  'context.trust_progressed':
    'La lista de revocaci\u00f3n de la aceptaci\u00f3n es posterior a la del registro, como corresponde.',
  'context.trust_order':
    'La lista de revocaci\u00f3n de la aceptaci\u00f3n no es posterior a la del registro.',
  'context.trust_unavailable': 'No se pudo leer una lista de revocaci\u00f3n del paquete.',
  'context.statements_unavailable': 'No se pudieron leer las declaraciones firmadas.',
  'context.document_after_manifest':
    'El sello de tiempo del documento es posterior al sello de tiempo del manifiesto.',
  'context.manifest_after_acceptance':
    'El sello de tiempo del manifiesto es posterior a la aceptaci\u00f3n de la firma.',
  'context.binding_registration_mismatch':
    'El manifiesto y el resumen de la firma indican fechas distintas de registro del certificado.',
};

export const signingFallbacks = {
  signature: 'No se pudo revisar la firma del documento.',
  binding: 'La vinculaci\u00f3n del certificado con la cuenta tiene un problema.',
  consent: 'El consentimiento firmado tiene un problema.',
  statement: STATEMENT_UNREADABLE,
  statements: 'Las declaraciones firmadas no coinciden entre s\u00ed.',
  context: 'Las fechas o registros del paquete no son consistentes.',
};
