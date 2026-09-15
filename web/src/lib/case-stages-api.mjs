const messages = {
  case_stage_conflict: 'La etapa cambi\u00f3. Consulta el registro actual y conserva tu borrador.',
  case_stage_required: 'Primero registra la etapa conocida del expediente.',
  case_stage_transition_rejected:
    'Este avance no corresponde a la etapa actual. Consulta su registro.',
  case_stage_profile_incomplete: 'Completa la ficha penal antes de registrar una etapa.',
  case_stage_revision_exhausted: 'Se alcanz\u00f3 el l\u00edmite de revisiones de etapa.',
  stage_support_changed:
    'El soporte cambi\u00f3 durante la validaci\u00f3n. Consulta de nuevo la versi\u00f3n elegida.',
  stage_support_digest_mismatch:
    'El digest no coincide con la versi\u00f3n. Selecciona de nuevo el soporte.',
  stage_support_too_large: 'El soporte supera el tama\u00f1o admitido para validar una etapa.',
  stage_support_format_rejected: 'El soporte no cumple el formato PDF o DOCX admitido para etapas.',
  stage_support_validation_limit:
    'El soporte supera los l\u00edmites de validaci\u00f3n del formato.',
  invalid_case_stage: 'Selecciona una etapa v\u00e1lida.',
  invalid_declared_stage_time: 'Revisa la fecha, hora y desfase UTC declarados.',
  invalid_stage_note: 'Revisa el motivo o la nota; admite hasta 1000 caracteres.',
  invalid_stage_court: 'Revisa el tribunal receptor; admite hasta 200 caracteres.',
  invalid_stage_receipt_reference:
    'Revisa la referencia de recepci\u00f3n; admite hasta 200 caracteres.',
  invalid_stage_act_order: 'Revisa el orden declarado de emisi\u00f3n y recepci\u00f3n.',
  conflicting_stage_support:
    'Una misma versi\u00f3n debe conservar el mismo digest en ambos soportes.',
  stage_act_in_future: 'La fecha declarada no puede estar completamente en el futuro.',
};
export function caseStagesApi(request, id) {
  let active = true;
  const base = `/cases/${encodeURIComponent(id)}/stage`;
  const assertActive = () => {
    if (!active) throw new Error('El expediente de esta solicitud ya no est\u00e1 abierto.');
  };
  async function call(suffix = '', options) {
    assertActive();
    let result;
    try {
      result = await request(`${base}${suffix}`, options);
    } catch (error) {
      if (messages[error.code]) error.message = messages[error.code];
      throw error;
    }
    assertActive();
    if (
      (result.case_id !== undefined && result.case_id !== id) ||
      (result.current && result.current.case_id !== id) ||
      result.entries?.some((entry) => entry.case_id !== id)
    )
      throw new Error('La etapa no corresponde al expediente abierto.');
    return result;
  }
  return {
    dispose: () => {
      active = false;
    },
    get: () => call(),
    history: ({ limit = 20, beforeRevision } = {}) => {
      const query = new URLSearchParams({ limit });
      if (beforeRevision !== undefined) query.set('before_revision', beforeRevision);
      return call(`/history?${query}`);
    },
    adopt: (data) => call('/adoption', { method: 'POST', data }),
    transition: (data) => call('/transitions', { method: 'POST', data }),
  };
}
