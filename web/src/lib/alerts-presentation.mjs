import { deadlineInstantLabel } from './deadline-time.mjs';

export function canAlerts(role) {
  return ['owner', 'litigator', 'paralegal'].includes(role);
}
export const alertPreferenceGroups = [
  { key: 'hearing_upcoming', label: 'Audiencias pr\u00f3ximas', noun: 'audiencias', family: true },
  { key: 'deadline_upcoming', label: 'Plazos pr\u00f3ximos', noun: 'plazos', family: true },
  {
    key: 'overdue_unattended',
    label: 'Vencimiento sin atenci\u00f3n declarada',
    noun: 'vencimiento sin atenci\u00f3n',
  },
  { key: 'review_required', label: 'Revisi\u00f3n requerida', noun: 'revisi\u00f3n requerida' },
  { key: 'due_changed_soon', label: 'Cambio de fecha pr\u00f3xima', noun: 'cambio de fecha' },
];
export function alertKindLabel(row) {
  if (row.kind.kind === 'upcoming')
    return row.subject.kind === 'hearing' ? 'Audiencia pr\u00f3xima' : 'Plazo pr\u00f3ximo';
  return {
    overdue_unattended: 'Vencido sin atenci\u00f3n declarada',
    review_required: 'Revisi\u00f3n requerida',
    due_changed_soon: 'Fecha pr\u00f3xima modificada',
  }[row.kind.kind];
}
export const alertEmailLabels = {
  disabled: 'Correo deshabilitado',
  pending: 'Correo pendiente',
  sending: 'Env\u00edo en curso',
  accepted: 'Aceptado por proveedor',
  failed: 'Env\u00edo rechazado',
  unknown: 'Resultado de correo incierto',
  cancelled: 'Correo cancelado',
};
export const alertResolutionLabels = {
  superseded: 'Sustituida por un cambio posterior',
  attention_recorded: 'Atenci\u00f3n declarada',
  target_retired: 'Plazo retirado',
  cancelled_hearing: 'Audiencia cancelada',
  no_longer_eligible: 'Ya no cumple las condiciones del aviso',
};
export const alertTimeLabel = (value) => deadlineInstantLabel(value).replace('.000000000', '');
export function alertFailure(error) {
  return (
    {
      alert_not_found: 'La alerta ya no esta disponible para tu cuenta.',
      permission_denied: 'Tu cuenta ya no tiene acceso a esta consulta.',
      alert_revision_conflict:
        'Las preferencias cambiaron. Consulta los valores actuales y compara tus cambios.',
      alert_operation_conflict:
        'Esta operacion no coincide con el guardado registrado. Consulta las preferencias actuales.',
      invalid_alert: 'Revisa las horas, los canales y los filtros de alertas.',
    }[error?.code] ||
    error?.message ||
    'No fue posible completar la consulta de alertas.'
  );
}
