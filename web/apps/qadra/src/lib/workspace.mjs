import { canCalendars } from './judicial-calendar-labels.mjs';
import { canHearings } from './hearings.mjs';
import { canParticipants } from './participants.mjs';
import { canAlerts } from './alerts-presentation.mjs';
import { contractDocumentStatus } from './contract-presentation.mjs';

// A client account reaches only its own contracts, its signing binding and
// the local evidence validator. Rationale:
// docs/adr/0086-bilateral-contract-signing.md.
export const clientViews = ['contracts', 'signing-certificates', 'evidence-validation'];

export function documentStatus(document) {
  const contract = contractDocumentStatus(document);
  if (contract) return contract;
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
  if (role === 'client') return clientViews.includes(view) ? view : 'contracts';
  if (['signing-certificates', 'contracts'].includes(view) && ['owner', 'litigator'].includes(role))
    return view;
  if (view === 'case-contracts' && ['owner', 'litigator', 'paralegal'].includes(role)) return view;
  if (view === 'reports' && ['owner', 'litigator'].includes(role)) return view;
  if (view === 'alerts' && canAlerts(role)) return view;
  if (view === 'judicial-calendars' && canCalendars(role)) return view;
  if (['hearings', 'agenda'].includes(view) && canHearings(role, 'read')) return view;
  if (
    ['participants', 'stages', 'resolutions', 'resources', 'deadlines', 'case-measures'].includes(
      view,
    ) &&
    canParticipants(role, 'read')
  )
    return view;
  if (['overview', 'cases', 'case-summary', 'documents', 'guide'].includes(view)) return view;
  if (
    role === 'owner' &&
    ['team', 'audit', 'integrity-incidents', 'case-members', 'owner-certificates'].includes(view)
  )
    return view;
  return 'overview';
}

export const alertReturnViews = [
  'case-summary',
  'documents',
  'participants',
  'stages',
  'hearings',
  'case-measures',
  'resolutions',
  'resources',
  'deadlines',
];

export const viewLabels = {
  overview: 'Inicio',
  reports: 'Informes',
  'case-summary': 'Expedientes / Resumen',
  'case-members': 'Expedientes / Asignaciones',
  cases: 'Expedientes',
  documents: 'Documentos',
  participants: 'Expedientes / Participantes',
  stages: 'Expedientes / Etapas',
  hearings: 'Expedientes / Audiencias',
  'case-measures': 'Expedientes / Medidas cautelares',
  resolutions: 'Expedientes / Resoluciones',
  resources: 'Expedientes / Recursos',
  deadlines: 'Expedientes / Plazos',
  'case-contracts': 'Expedientes / Contratos',
  contracts: 'Contratos',
  'evidence-validation': 'Validar evidencia',
  agenda: 'Agenda',
  alerts: 'Mis alertas',
  'integrity-incidents': 'Incidentes de integridad',
  'judicial-calendars': 'Calendarios jurisdiccionales',
  team: 'Equipo',
  'signing-certificates': 'Mi firma digital',
  'owner-certificates': 'Mi certificado',
  audit: 'Auditor\u00eda',
  guide: 'Gu\u00eda de uso',
};
