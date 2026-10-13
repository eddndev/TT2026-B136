export function base64Bytes(value) {
  return Uint8Array.from(atob(value), (character) => character.charCodeAt(0));
}
export async function bytesBase64(blob) {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  let value = '';
  for (const byte of bytes) value += String.fromCharCode(byte);
  return btoa(value);
}
export function matchesSubmission(record, prepared, signature, evidence) {
  if (record.id !== prepared.proposal.participant_id) return false;
  if (!prepared.declaration)
    return (
      record.submission_revision === prepared.submission_revision &&
      record.submission_digest === prepared.submission_digest &&
      typeof prepared.submission_digest === 'string'
    );
  const origin = record.credential_origin,
    declaration = prepared.declaration;
  return (
    !!origin &&
    origin.participant_id === record.id &&
    origin.participant_revision === prepared.submission_revision &&
    origin.statement_digest === declaration.digest &&
    !!evidence &&
    evidence.reference?.participant_id === record.id &&
    evidence.reference?.participant_revision === prepared.submission_revision &&
    evidence.statement_digest === declaration.digest &&
    evidence.declaration_base64 === declaration.bytes_base64 &&
    evidence.certificate_fingerprint === declaration.certificate.fingerprint &&
    evidence.signature_base64 === signature
  );
}
const errors = {
  participant_credential_revoked:
    'El certificado estaba revocado en la confianza comprobada. Revisa la evidencia y selecciona un certificado admitido antes de preparar otra declaraci\u00f3n.',
  participant_credential_invalid_signature:
    'La firma no corresponde a la declaraci\u00f3n y al certificado seleccionados. Firma el archivo binario exacto con la herramienta del titular.',
  participant_credential_expired:
    'El certificado no est\u00e1 dentro de su intervalo de validez para este registro.',
  participant_credential_not_yet_valid:
    'El certificado todav\u00eda no es v\u00e1lido para este registro.',
  participant_credential_untrusted_issuer:
    'El certificado no est\u00e1 admitido por la confianza de la CA interna.',
  participant_credential_malformed_certificate: 'El certificado no tiene el formato admitido.',
  participant_credential_unsupported_certificate:
    'El certificado no usa el perfil admitido para declaraciones personales.',

  participant_credential_malformed_crl:
    'La lista de revocaci\u00f3n publicada no tiene el formato admitido. Solicita al administrador que revise la confianza de la CA interna.',
  participant_credential_unsupported_crl:
    'La lista de revocaci\u00f3n publicada no usa el perfil admitido. Solicita al administrador que revise la confianza de la CA interna.',
  participant_credential_untrusted_crl:
    'La lista de revocaci\u00f3n no corresponde a la CA interna. Solicita al administrador que revise la confianza publicada.',
  participant_credential_crl_not_yet_valid:
    'La lista de revocaci\u00f3n publicada todav\u00eda no es v\u00e1lida. Solicita al administrador que revise la confianza de la CA interna.',
  participant_credential_crl_expired:
    'La lista de revocaci\u00f3n publicada est\u00e1 vencida. Solicita al administrador que publique una lista vigente antes de preparar otra declaraci\u00f3n.',
  participant_credential_limit_exceeded:
    'El material de comprobaci\u00f3n supera los l\u00edmites admitidos. Revisa el certificado seleccionado y solicita al administrador que revise la confianza publicada.',

  participant_candidate_limit:
    'La consulta supera la capacidad de 16 candidatos. No cambies identificadores para evitar coincidencias. Este registro requiere resolver la capacidad del directorio antes de continuar.',
  participant_role_conflict:
    'Esta identidad ya tiene una ficha de ese tipo, incluso si est\u00e1 archivada. Consulta el directorio completo para revisarla.',
  participant_identity_review_conflict:
    'El directorio cambi\u00f3. Conserva tu formulario y vuelve a revisar las coincidencias.',
  participant_identity_review_required:
    'Revisa todos los candidatos y explica la selecci\u00f3n de identidad.',
  subject_revision_conflict:
    'La identidad cambi\u00f3. Consulta los datos actuales y comp\u00e1ralos antes de preparar otra declaraci\u00f3n.',
  participant_revision_conflict:
    'La ficha cambi\u00f3. Consulta los datos actuales y comp\u00e1ralos antes de preparar otro registro.',
  participant_support_changed:
    'El soporte cambi\u00f3 o ya no est\u00e1 disponible. Consulta de nuevo su versi\u00f3n exacta; la carga confirmada se conserva.',
  participant_credential_required:
    'Este tipo requiere certificado p\u00fablico y firma personal de la declaraci\u00f3n.',
  participant_credential_unexpected:
    'No se admite una firma personal para un \u00f3rgano institucional.',
  participant_subject_change_forbidden:
    'Una ficha tipificada conserva su identidad vinculada. No se puede sustituir por otra.',
  credential_trust_unavailable:
    'La confianza de la CA interna no est\u00e1 disponible para comprobar la firma.',
  credential_trust_changed:
    'La confianza publicada cambi\u00f3. Conserva el env\u00edo y consulta su resultado antes de confirmar de nuevo.',
};
export function typedParticipantFailure(failure) {
  if (errors[failure.code]) return errors[failure.code];
  if (failure.code?.endsWith('_revision_exhausted'))
    return 'Se agotaron las revisiones disponibles. Consultar de nuevo no permite agregar otra revisi\u00f3n.';
  if (!failure.status || failure.status >= 500)
    return 'No se pudo confirmar el resultado. Conservamos el env\u00edo; consulta su revisi\u00f3n exacta antes de volver a enviarlo.';
  if (failure.status === 413)
    return 'El registro supera el l\u00edmite admitido. El certificado p\u00fablico debe ocupar hasta 16 KiB.';
  return failure.message || 'No se pudo registrar el participante. Revisa los datos y el soporte.';
}

export async function readSubmission(api, bundle, valid) {
  const prepared = bundle.prepared;
  let record;
  try {
    record = await api.participantRevision(
      prepared.proposal.participant_id,
      prepared.submission_revision,
    );
  } catch (failure) {
    if (failure.status === 404 && failure.code === 'participant_not_found')
      return { state: 'absent' };
    throw failure;
  }
  if (!valid()) return { state: 'obsolete' };
  const origin = record.credential_origin;
  const needsEvidence =
    prepared.declaration &&
    origin?.participant_revision === prepared.submission_revision &&
    origin.statement_digest === prepared.declaration.digest;
  const evidence = needsEvidence
    ? await api.credential(record.id, prepared.submission_revision)
    : null;
  return {
    state: matchesSubmission(record, prepared, bundle.request.signature_base64, evidence)
      ? 'matched'
      : 'different',
    record,
  };
}
