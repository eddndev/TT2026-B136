import { canParticipants } from './participants.mjs';
export function documentStatus(document) {
  if (document.report?.verdict === 'not_valid')
    return { label: 'Revisar evidencia', tone: 'danger', key: 'failed' };
  if (document.report?.verdict === 'valid')
    return { label: 'Verificado', tone: 'success', key: 'verified' };
  if (document.sealed === true) return { label: 'Sellado', tone: 'info', key: 'sealed' };
  if (document.sealed === false)
    return { label: 'Pendiente de sello', tone: 'warning', key: 'pending' };
  return { label: 'Estado por confirmar', tone: 'neutral', key: 'unknown' };
}

export function normalizeView(hash, role) {
  const view = hash.replace(/^#/, '');
  if (view === 'participants' && canParticipants(role, 'read')) return view;
  if (['overview', 'cases', 'documents', 'guide'].includes(view)) return view;
  if (role === 'owner' && ['team', 'audit'].includes(view)) return view;
  return 'overview';
}

export const viewLabels = {
  overview: 'Inicio',
  cases: 'Expedientes',
  documents: 'Documentos',
  participants: 'Expedientes / Participantes',
  team: 'Equipo',
  audit: 'Auditor\u00eda',
  guide: 'Gu\u00eda de uso',
};
