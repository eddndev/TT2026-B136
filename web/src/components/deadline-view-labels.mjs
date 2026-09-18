export const deadlineActions = {
  register: 'Registro',
  correct: 'Correcci\u00f3n',
  set_attention: 'Atenci\u00f3n declarada',
  retire: 'Retiro',
};
export const deadlineFields = {
  resolution_issued_at: 'Emisi\u00f3n de la resoluci\u00f3n',
  notification_practiced_at: 'Pr\u00e1ctica de la notificaci\u00f3n',
  notification_received_at: 'Recepci\u00f3n de la notificaci\u00f3n',
  notification_stated_effect_at: 'Efecto declarado de la notificaci\u00f3n',
  hearing_session_event_time: 'Momento declarado de la audiencia',
};
export const deadlineFamilies = {
  resolution: 'Resoluci\u00f3n',
  notification: 'Notificaci\u00f3n',
  hearing_result: 'Resultado de audiencia',
};
export const deadlinePurposes = {
  hearing_end: 'Fin declarado de audiencia',
  ordered_period_start: 'Inicio declarado del periodo concedido',
};
export function deadlineRequirementLabel(value) {
  if (value.kind === 'source_field') return deadlineFields[value.field];
  return `${deadlinePurposes[value.purpose]} / ${deadlineFamilies[value.family]}`;
}
export function deadlineRuleLabel(value) {
  if (!value) return 'Regla pendiente de determinar';
  const quantity = value.quantity ?? 'Cantidad concedida';
  if (value.kind === 'elapsed_hours') return `${quantity} horas transcurridas`;
  const final =
    value.final_day === 'next_countable'
      ? 'ajuste al siguiente d\u00eda computable'
      : 'conserva la fecha candidata';
  if (value.kind === 'civil_months') return `${quantity} meses civiles; ${final}`;
  return `${quantity} d\u00edas ${value.basis === 'natural' ? 'naturales' : 'computables seg\u00fan calendario'}; ${value.inclusion === 'after_anchor' ? 'desde el d\u00eda siguiente' : 'incluye el d\u00eda de inicio'}; ${final}`;
}
export function deadlineBlockLabel(value) {
  if (['trigger', 'arithmetic', 'rule'].includes(value.kind))
    return deadlineBlockLabel(value.block);
  const simple = {
    scope_unknown: 'No se conoce si el perfil aplica al expediente.',
    scope_rejected: 'Se declar\u00f3 que el perfil no aplica al expediente.',
    incident_unknown: 'No se conoce si existe una incidencia pendiente.',
    unresolved_incident: 'Existe una incidencia pendiente de resolver.',
    civil_cutoff_missing:
      'Hay fecha candidata, pero falta una hora de corte para fijar el vencimiento.',
    missing_ordered_quantity: 'Falta declarar la duraci\u00f3n efectivamente concedida.',
    unexpected_ordered_quantity:
      'Se envi\u00f3 una duraci\u00f3n concedida a una regla de cantidad fija.',
    unknown_source: 'La fuente del inicio es desconocida.',
    unexpected_qualification: 'La regla no admite esta calificaci\u00f3n adicional del inicio.',
    unknown_anchor: 'No se conoce el momento de inicio.',
    missing_offset: 'Falta el desfase horario del inicio.',
    missing_calendar: 'La regla necesita un calendario seleccionado.',
    date_range_exhausted: 'El c\u00f3mputo rebasa el intervalo de fechas admitido.',
  };
  if (simple[value.kind]) return simple[value.kind];
  switch (value.kind) {
    case 'condition_missing':
      return `Falta responder la condici\u00f3n ${value.id}.`;
    case 'condition_unknown':
      return `No se conoce la respuesta a la condici\u00f3n ${value.id}.`;
    case 'condition_rejected':
      return `La condici\u00f3n ${value.id} no se cumple seg\u00fan lo declarado.`;
    case 'cutoff_outside_coverage':
      return `La fecha ${value.candidate} queda fuera de la cobertura de la hora de corte.`;
    case 'ordered_quantity_exceeds_maximum':
      return `La duraci\u00f3n concedida ${value.supplied} supera el m\u00e1ximo ${value.maximum} del perfil.`;
    case 'absent_field':
      return `La fuente no contiene ${deadlineFields[value.field]}.`;
    case 'incompatible_family':
      return `Se necesita ${deadlineFamilies[value.expected]}; se seleccion\u00f3 ${deadlineFamilies[value.actual]}.`;
    case 'missing_qualification':
      return `Falta ${deadlinePurposes[value.purpose]}.`;
    case 'qualification_mismatch':
      return `Se requiere ${deadlinePurposes[value.expected]}; se declar\u00f3 ${deadlinePurposes[value.actual]}.`;
    case 'insufficient_precision':
      return `La precisi\u00f3n del inicio (${{ unknown: 'desconocida', date: 'fecha', minute: 'minuto', second: 'segundo' }[value.observed]}) no permite este c\u00f3mputo.`;
    case 'missing_homologous_day':
      return `El d\u00eda ${value.requested_day} no existe en ${String(value.year).padStart(4, '0')}-${String(value.month).padStart(2, '0')}; no se sustituye por otro d\u00eda.`;
    case 'unresolved_calendar_date':
      return `El calendario deja sin resolver la fecha ${value.date}.`;
    case 'outside_calendar_coverage':
      return `La fecha ${value.date} queda fuera de la cobertura del calendario.`;
    default:
      return 'No se reconoce el motivo del bloqueo recibido.';
  }
}
export function deadlineDeclarationLabel(value) {
  return value.kind === 'known' ? (value.value ? 'S\u00ed' : 'No') : `Desconocido: ${value.reason}`;
}
