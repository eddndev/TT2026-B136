export const canReports = (role) => ['owner', 'litigator'].includes(role);
export const reportScope = (role) => (role === 'owner' ? 'office' : 'assigned_cases');
export const scopeLabel = (scope) =>
  scope === 'office' ? 'Todo el despacho' : 'Tus expedientes asignados';
export const statusLabel = (value) =>
  ({ all: 'Todos', active: 'Activos', closed: 'Cerrados' })[value] || value;
export function phaseLabel(value) {
  if (value.state === 'queued') return 'En cola';
  if (value.state === 'processing')
    return value.phase === 'capturing' ? 'Capturando expedientes' : 'Generando PDF y CSV';
  if (value.state === 'retry_waiting') return 'Reintento pendiente';
  if (value.state === 'ready') return 'Disponible';
  if (value.state === 'access_revoked') return 'Acceso retirado';
  return 'No se pudo generar el informe';
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
    from: from.toISOString().slice(0, 10),
    before: before.toISOString().slice(0, 10),
    status: 'all',
    assigned: '',
  };
}
function utcDay(value) {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) throw new Error('Indica ambas fechas de creaci\u00f3n.');
  const date = new Date(`${value}T00:00:00Z`);
  if (!Number.isFinite(date.getTime()) || date.toISOString().slice(0, 10) !== value)
    throw new Error('La fecha de creaci\u00f3n no es v\u00e1lida.');
  return date;
}
export function reportFilters(draft, lawyers) {
  const from = utcDay(draft.from),
    before = utcDay(draft.before);
  const duration = before.getTime() - from.getTime();
  if (duration <= 0 || duration > 366 * 86400000)
    throw new Error('El intervalo debe ser positivo y no superar 366 d\u00edas.');
  if (!['all', 'active', 'closed'].includes(draft.status))
    throw new Error('El estado administrativo no es v\u00e1lido.');
  if (draft.assigned && !lawyers.some((row) => row.user_id === draft.assigned))
    throw new Error('Selecciona un litigante disponible en tu alcance.');
  return {
    created_from: from.toISOString().replace('.000Z', 'Z'),
    created_before: before.toISOString().replace('.000Z', 'Z'),
    status: draft.status,
    assigned_litigator: draft.assigned || null,
  };
}
export const reportFailure = (error) =>
  error?.status === 403
    ? 'Ya no tienes acceso a este informe. Actualiza las solicitudes disponibles.'
    : error?.status === 404
      ? 'Este informe ya no est\u00e1 disponible para tu cuenta.'
      : error?.message || 'No fue posible consultar los informes.';
