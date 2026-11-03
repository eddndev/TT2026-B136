export const messages = {
  audit_query_capacity_exceeded:
    'La p\u00e1gina de actividad supera su capacidad. Reduce el n\u00famero de eventos o concreta los filtros.',
  invalid_audit_query:
    'Revisa el periodo, los filtros exactos y el tama\u00f1o de la p\u00e1gina de actividad.',
  case_closed:
    'El expediente est\u00e1 cerrado administrativamente. Consulta su estado antes de modificarlo.',
  case_revision_conflict: 'Los datos del expediente cambiaron. Consulta los valores actuales.',
  case_revision_exhausted:
    'El expediente alcanz\u00f3 el l\u00edmite de revisiones. No se pueden guardar m\u00e1s cambios.',
  case_identifier_conflict:
    'El NUC o la carpeta judicial ya est\u00e1n registrados. Revisa ambos identificadores.',
  case_profile_required: 'La ficha penal completa debe conservarse. Consulta los valores actuales.',
  invalid_penal_case_profile:
    'Revisa los campos obligatorios, las descripciones y los l\u00edmites de la ficha penal.',
  invalid_case_metadata: 'Revisa el t\u00edtulo y la referencia interna.',
  case_administration_body_too_large: 'Los datos del expediente superan el l\u00edmite permitido.',

  invalid_participant_values: 'Revisa los campos del participante.',
  invalid_participant_revision: 'La revisi\u00f3n del participante no es v\u00e1lida.',
  participant_not_found: 'El participante no est\u00e1 disponible en este expediente.',
  participant_revision_exhausted:
    'El participante alcanz\u00f3 el l\u00edmite de revisiones. No se pueden guardar m\u00e1s cambios.',
  invalid_credentials: 'Correo o contrase\u00f1a incorrectos.',
  mfa_rejected: 'El c\u00f3digo fue rechazado o venci\u00f3. Vuelve a iniciar sesi\u00f3n.',
  invalid_session: 'Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.',
  permission_denied: 'No tienes permiso para realizar esta acci\u00f3n.',
  account_locked: 'Demasiados intentos. Espera 15 minutos antes de volver a intentarlo.',
  document_not_found: 'No se encontr\u00f3 un documento con ese identificador.',
  document_format_unsupported:
    'El formato del archivo no est\u00e1 admitido. Elige un archivo de un formato compatible.',
  document_format_invalid:
    'El archivo no es v\u00e1lido o su contenido est\u00e1 da\u00f1ado. Revisa el archivo o elige otro.',
  document_validation_limit:
    'El archivo supera el l\u00edmite de validaci\u00f3n. Elige un archivo de menor complejidad o duraci\u00f3n.',
  document_validator_unavailable:
    'El validador no est\u00e1 disponible temporalmente. Conserva el archivo e int\u00e9ntalo de nuevo cuando est\u00e9 disponible.',
  document_version_required:
    'Selecciona una versi\u00f3n del historial para realizar esta acci\u00f3n.',
  document_version_exhausted:
    'Este documento ha alcanzado el l\u00edmite de versiones. Conserva tu archivo y c\u00e1rgalo como un documento nuevo.',
  document_already_sealed: 'Este documento ya est\u00e1 sellado.',
  document_not_sealed: 'Primero sella el documento para verificarlo o descargar su evidencia.',
  document_content_validation_failed:
    'No se descarg\u00f3 el archivo: su validaci\u00f3n fall\u00f3.',
  document_integrity_incident_not_found: 'El incidente de integridad no est\u00e1 disponible.',
  user_already_exists:
    'Ya existe una cuenta con ese correo. Usa otro correo para el nuevo integrante.',
  bootstrap_closed: 'El despacho ya tiene un administrador. Inicia sesi\u00f3n con tu cuenta.',
  invalid_input: 'Revisa los datos ingresados y los l\u00edmites de cada campo.',
  case_not_found: 'El expediente no est\u00e1 disponible o ya no tienes acceso.',
  user_not_found: 'No se encontr\u00f3 un usuario activo con ese identificador.',
  user_revision_conflict:
    'La cuenta cambi\u00f3. Consulta su revisi\u00f3n actual antes de confirmar.',
  last_active_owner:
    'Debe conservarse al menos un administrador activo; no puedes retirar al \u00faltimo.',
  user_access_version_exhausted: 'La cuenta alcanz\u00f3 el l\u00edmite de cambios de acceso.',
  invalid_document_metadata: 'Revisa el tipo, la clasificaci\u00f3n y las etiquetas ingresadas.',
  document_metadata_revision_exhausted:
    'Este documento ha alcanzado el l\u00edmite de cambios de clasificaci\u00f3n. No se pueden registrar m\u00e1s cambios.',
  invalid_document_name: 'El nombre del documento no es v\u00e1lido.',
};
