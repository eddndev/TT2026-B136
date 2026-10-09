import { reportInstant, reportValue } from './case-report-values.mjs';

export const activityReport = (value) => value?.report_type === 'litigator_activity';
export const reportTypeLabel = (value) =>
  activityReport(value) ? 'Actividad por litigante' : 'Estado y carga de expedientes';
export const reportPeriod = (value) => ({
  from: activityReport(value) ? value.filters.occurred_from : value.filters.created_from,
  before: activityReport(value) ? value.filters.occurred_before : value.filters.created_before,
});
export const reportAuthor = (value) =>
  activityReport(value) ? value.filters.author_litigator : value.filters.assigned_litigator;
export const canReports = (role) => ['owner', 'litigator'].includes(role);
export const reportScope = (role) => (role === 'owner' ? 'office' : 'assigned_cases');
export const scopeLabel = (scope) =>
  scope === 'office' ? 'Todo el despacho' : 'Tus expedientes asignados';
export const statusLabel = (value) =>
  ({ all: 'Todos', active: 'Activos', closed: 'Cerrados' })[value] || value;
export function phaseLabel(value) {
  if (value.state === 'queued') return 'En cola';
  if (value.state === 'processing')
    return value.phase === 'capturing'
      ? activityReport(value)
        ? 'Capturando actividad'
        : 'Capturando expedientes'
      : 'Generando PDF y CSV';
  if (value.state === 'retry_waiting') return 'Reintento pendiente';
  if (value.state === 'ready') return 'Disponible';
  if (value.state === 'access_revoked') return 'Acceso retirado';
  return 'No se pudo generar el informe';
}

function reportTimeNanoseconds(value) {
  const [seconds, fraction = ''] = value.slice(0, -1).split('.');
  return BigInt(reportInstant(`${seconds}Z`)) * 1_000_000n + BigInt(fraction.padEnd(9, '0'));
}

export function reportDuration(value) {
  try {
    reportValue(value);
    if (!['ready', 'failed'].includes(value.state)) return null;
    const requested = reportTimeNanoseconds(value.requested_at),
      finished = reportTimeNanoseconds(value.notice.created_at),
      updated = reportTimeNanoseconds(value.updated_at),
      captured = value.ready ? reportTimeNanoseconds(value.ready.checked_at) : requested,
      read = value.notice.read_at === null ? finished : reportTimeNanoseconds(value.notice.read_at);
    if (requested > captured || captured > finished || finished > read || read > updated)
      return null;
    let remaining = (finished - requested) / 1_000_000_000n;
    if (remaining === 0n) return 'Menos de 1 s';
    const parts = [];
    for (const [seconds, label] of [
      [86400n, 'd'],
      [3600n, 'h'],
      [60n, 'min'],
      [1n, 's'],
    ]) {
      const count = remaining / seconds;
      if (count > 0n) parts.push(`${count} ${label}`);
      remaining %= seconds;
    }
    return parts.join(' ');
  } catch {
    return null;
  }
}

export const failureLabel = (code) =>
  ({
    capacity_exceeded:
      'El informe supera el l\u00edmite permitido. Solicita otro con un periodo o filtro m\u00e1s reducido.',
    render_unavailable: 'El generador no est\u00e1 disponible. La solicitud conserva su estado.',
    render_failed:
      'No fue posible representar todo el contenido. No se publicaron archivos parciales.',
    temporarily_unavailable: 'El servicio no pudo completar el trabajo en este momento.',
    invalid_capture: 'No fue posible validar la captura del informe.',
    access_revoked: 'Tu acceso a este informe cambi\u00f3.',
  })[code] || 'No fue posible completar el informe.';
export function initialReportFilters() {
  const now = new Date();
  const from = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1));
  const before = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate() + 1));
  return {
    reportType: 'case_state',
    from: from.toISOString().slice(0, 10),
    before: before.toISOString().slice(0, 10),
    status: 'all',
    assigned: '',
  };
}
function utcDay(value, activity) {
  const label = activity ? 'actividad' : 'creaci\u00f3n';
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) throw new Error(`Indica ambas fechas de ${label}.`);
  const date = new Date(`${value}T00:00:00Z`);
  if (!Number.isFinite(date.getTime()) || date.toISOString().slice(0, 10) !== value)
    throw new Error(`La fecha de ${label} no es v\u00e1lida.`);
  return date;
}
export function reportFilters(draft, lawyers) {
  if (![undefined, 'case_state', 'litigator_activity'].includes(draft.reportType))
    throw new Error('Selecciona un tipo de informe disponible.');
  const activity = draft.reportType === 'litigator_activity',
    from = utcDay(draft.from, activity),
    before = utcDay(draft.before, activity);
  const duration = before.getTime() - from.getTime();
  if (duration <= 0 || duration > 366 * 86400000)
    throw new Error('El intervalo debe ser positivo y no superar 366 d\u00edas.');
  if (!['all', 'active', 'closed'].includes(draft.status))
    throw new Error('El estado administrativo no es v\u00e1lido.');
  if (draft.assigned && !lawyers.some((row) => row.user_id === draft.assigned))
    throw new Error('Selecciona un litigante disponible en tu alcance.');
  return {
    [activity ? 'occurred_from' : 'created_from']: from.toISOString().replace('.000Z', 'Z'),
    [activity ? 'occurred_before' : 'created_before']: before.toISOString().replace('.000Z', 'Z'),
    status: draft.status,
    [activity ? 'author_litigator' : 'assigned_litigator']: draft.assigned || null,
  };
}
export const reportFailure = (error) =>
  error?.status === 403
    ? 'Ya no tienes acceso a este informe. Actualiza las solicitudes disponibles.'
    : error?.status === 404
      ? 'Este informe ya no est\u00e1 disponible para tu cuenta.'
      : error?.message || 'No fue posible consultar los informes.';
