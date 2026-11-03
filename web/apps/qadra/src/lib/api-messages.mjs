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
  invalid_contract_request:
    'La solicitud del contrato no tiene el formato esperado. Revisa los datos y vuelve a intentarlo.',
  contract_not_found: 'El contrato no est\u00e1 disponible o ya no tienes acceso.',
  contract_revision_conflict:
    'El contrato cambi\u00f3 mientras lo revisabas. Consulta su estado actual antes de continuar.',
  contract_exists: 'Este documento ya tiene un contrato. Abre el existente en Contratos.',
  contract_request_open:
    'Ya hay una solicitud de firma abierta. Canc\u00e9lala o espera a que venza antes de pedir otra.',
  contract_request_closed: 'La solicitud ya est\u00e1 cerrada y no admite m\u00e1s acciones.',
  contract_out_of_turn:
    'No es tu turno de firmar. Primero firma el Cliente y despu\u00e9s el despacho.',
  contract_expired: 'La vigencia de la solicitud termin\u00f3. El despacho debe pedir una nueva.',
  contract_version_superseded:
    'Existe una versi\u00f3n posterior del documento. La solicitud ya no puede firmarse.',
  contract_version_frozen:
    'Esta versi\u00f3n est\u00e1 bajo contrato: no admite sello ni firma individual.',
  contract_counterparty_revoked:
    'El certificado del Cliente fue revocado antes de la firma del despacho. El contrato no puede completarse.',
  contract_not_complete: 'El paquete solicitado todav\u00eda no est\u00e1 disponible.',
  contract_body_too_large: 'Los datos del contrato superan el l\u00edmite permitido.',
  contract_party_ineligible:
    'Falta preparar a una de las partes o el documento. Revisa la lista de preparaci\u00f3n.',
  contract_timestamp_unavailable:
    'La autoridad de sellado de tiempo no respondi\u00f3 y no se registr\u00f3 nada. Puedes volver a enviar la misma firma.',
  signing_trust_unavailable:
    'La configuraci\u00f3n de confianza no est\u00e1 disponible y no se registr\u00f3 nada. Int\u00e9ntalo m\u00e1s tarde con la misma firma.',
  invalid_json: 'La solicitud no tiene el formato esperado.',
  signing_account_changed:
    'Tu cuenta cambi\u00f3 durante la firma. Vuelve a preparar la declaraci\u00f3n.',
  signing_trust_changed:
    'La configuraci\u00f3n de confianza cambi\u00f3. Vuelve a preparar la declaraci\u00f3n.',
  signing_conflict:
    'Esta operaci\u00f3n ya se envi\u00f3 con otros datos. Consulta su recibo antes de continuar.',
  signing_binding_inactive:
    'Tu firma documental no est\u00e1 vinculada o fue retirada. Rev\u00edsala en Mi firma.',
  signing_credential_rejected:
    'El certificado no es aceptado por la autoridad del despacho. Rev\u00edsalo en Mi firma.',
};
