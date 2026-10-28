// Spanish messages for the findings about certificates, revocation lists,
// the trust of their roots and the current status. The codes are the stable
// codes of crates/evidence-verify. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.

const STRICT_PROFILE = 'El certificado no cumple el perfil estricto.';
const CRL_PROFILE = 'La lista de revocaci\u00f3n no cumple el perfil estricto.';
const NOT_PINNED = 'La ra\u00edz no est\u00e1 en la lista fija de Qadra.';
const NO_CRL = 'Falta la lista de revocaci\u00f3n que cubre esa fecha.';

const each = (codes, text) => Object.fromEntries(codes.map((code) => [code, text]));

export const certificateMessages = {
  'cert.profile': 'El certificado cumple el perfil estricto.',
  ...each(
    [
      'cert.profile.authority_key_id',
      'cert.profile.basic_constraints',
      'cert.profile.empty_name',
      'cert.profile.extended_key_usage',
      'cert.profile.extensions',
      'cert.profile.key_algorithm',
      'cert.profile.key_exponent',
      'cert.profile.key_size',
      'cert.profile.key_usage',
      'cert.profile.self_signature',
      'cert.profile.serial',
      'cert.profile.signature_algorithm',
      'cert.profile.signature_length',
      'cert.profile.subject_key_id',
      'cert.profile.unique_ids',
      'cert.profile.validity',
      'cert.profile.version',
    ],
    STRICT_PROFILE,
  ),
  'cert.malformed': 'No se pudo leer un certificado del paquete.',
  'cert.too_large': 'Un certificado del paquete es m\u00e1s grande de lo permitido.',
  'cert.issued_by': 'El certificado fue emitido por la ra\u00edz del paquete.',
  'cert.issuer.key_id_mismatch': 'El certificado no fue emitido por la ra\u00edz del paquete.',
  'cert.issuer.name_mismatch': 'El certificado no fue emitido por la ra\u00edz del paquete.',
  'cert.issuer.signature_invalid': 'La firma de la ra\u00edz sobre el certificado es incorrecta.',
  'cert.validity': 'Los certificados estaban vigentes en la fecha revisada.',
  'cert.outside_validity': 'Un certificado no estaba vigente en la fecha revisada.',
  'cert.instant_unknown':
    'Falta evidencia para saber si los certificados estaban vigentes en esa fecha.',
};

export const crlMessages = {
  'crl.not_revoked': 'El certificado no estaba revocado en esa fecha.',
  'crl.revoked': 'El certificado estaba revocado en esa fecha.',
  'crl.revoked_later':
    'El certificado se revoc\u00f3 despu\u00e9s de la fecha revisada; eso no cambia este resultado.',
  'crl.not_covering': NO_CRL,
  'crl.malformed': 'No se pudo leer una lista de revocaci\u00f3n del paquete.',
  'crl.too_large': 'Una lista de revocaci\u00f3n del paquete es m\u00e1s grande de lo permitido.',
  'crl.issuer.key_id_mismatch':
    'La lista de revocaci\u00f3n no fue emitida por la ra\u00edz del paquete.',
  'crl.issuer.name_mismatch':
    'La lista de revocaci\u00f3n no fue emitida por la ra\u00edz del paquete.',
  'crl.issuer.signature_invalid': 'La firma de la lista de revocaci\u00f3n es incorrecta.',
  ...each(
    [
      'crl.profile.algorithm',
      'crl.profile.authority_key_id',
      'crl.profile.entry_extensions',
      'crl.profile.entry_limit',
      'crl.profile.entry_serial',
      'crl.profile.extensions',
      'crl.profile.next_update',
      'crl.profile.number',
      'crl.profile.revocation_date',
      'crl.profile.signature_length',
      'crl.profile.version',
    ],
    CRL_PROFILE,
  ),
  'revocation.certificate_unavailable': `${NO_CRL} No se pudo leer el certificado a revisar.`,
  'revocation.instant_unknown': `${NO_CRL} No se pudo establecer la fecha a revisar.`,
  'revocation.tsa_crl_unavailable': `${NO_CRL} Falta la de la autoridad del sello de tiempo.`,
  'revocation.tsa_unavailable': `${NO_CRL} La autoridad del sello de tiempo no est\u00e1 en la lista fija de Qadra.`,
};

export const trustMessages = {
  'trust.root_pinned': 'La ra\u00edz est\u00e1 en la lista fija de Qadra.',
  'trust.root_not_pinned': NOT_PINNED,
  'trust.served_not_pinned': `${NOT_PINNED} El servidor la usa actualmente.`,
  'trust.root_served_not_pinned': `${NOT_PINNED} El servidor la usa, pero eso no basta para confiar en ella.`,
  'trust.token_anchor_not_pinned': `${NOT_PINNED} Es la ra\u00edz de la autoridad del sello de tiempo.`,
  'trust.pinned_not_served': 'La ra\u00edz sigue fija pero el servidor ya no la usa.',
  'trust.route_unavailable':
    'No se pudo consultar la confianza actual del servidor. El resultado no cambia; solo el estado actual usa las listas del paquete.',
};

export const currentMessages = {
  'current.instant': 'El estado actual se calcul\u00f3 con el reloj de este equipo.',
  'current.within_validity': 'Hoy el certificado sigue vigente.',
  'current.outside_validity': 'Hoy el certificado ya no est\u00e1 vigente.',
  'current.no_crl': 'No hay una lista de revocaci\u00f3n actual para este certificado.',
  'current.served_crl_unreadable':
    'No se pudo leer la lista de revocaci\u00f3n actual del servidor.',
  'current.trust_route_unavailable':
    'No se pudo consultar al servidor; el estado actual usa las listas del paquete.',
};

export const certificateFallbacks = {
  cert: 'Un certificado del paquete tiene un problema.',
  crl: 'Una lista de revocaci\u00f3n del paquete tiene un problema.',
  revocation: NO_CRL,
  trust: NOT_PINNED,
  current: 'Dato informativo sobre el estado actual.',
};
