// How the evidence validator shows a report: plain labels for the overall
// result and each component, kept apart so that a correct signature, a
// certificate outside the strict profile, an unpinned root and missing
// evidence never read as the same thing. Rationale: the sections "Component
// results and the overall result" and "Trust anchors" of
// docs/adr/0085-in-browser-evidence-validation.md.
import { findingMessage } from './messages.mjs';

export { currentStatusView, serverComparison, signatureNote } from './notes.mjs';

/**
 * The largest local file the panel reads. The engine admits at most 25 MiB
 * of entries (MAX_TOTAL in crates/evidence-verify/src/archive/limits.rs);
 * one more MiB leaves room for the ZIP headers.
 */
export const MAX_EVIDENCE_FILE_BYTES = 26 * 1024 * 1024;

export const STAGE_LABELS = Object.freeze({
  read: 'Leyendo el archivo',
  trust: 'Consultando las ra\u00edces de confianza del servidor',
  module: 'Preparando el validador',
  validate: 'Revisando la evidencia',
});

export const PURPOSE_LABELS = Object.freeze({
  'server-seal': 'Sello del servidor',
  'individual-signing': 'Firma individual',
  'time-stamping': 'Sello de tiempo',
});

const PROFILE_LABELS = {
  'server-seal-v1': 'Sello del servidor',
  'individual-signing-v1': 'Firma individual, versi\u00f3n 1',
  'individual-signing-v2': 'Firma individual, versi\u00f3n 2',
};

const OVERALL = {
  checked: {
    label: 'Completo',
    tone: 'success',
    summary: 'Todas las comprobaciones obligatorias pasaron.',
  },
  indeterminate: {
    label: 'Parcial',
    tone: 'warning',
    summary: 'Ninguna comprobaci\u00f3n fall\u00f3, pero falta evidencia para decidir alguna.',
  },
  failed: {
    label: 'Fallido',
    tone: 'danger',
    summary: 'Al menos una comprobaci\u00f3n obligatoria fall\u00f3.',
  },
  'not-supported': {
    label: 'No compatible',
    tone: 'neutral',
    summary:
      'El archivo o alguna de sus partes no se puede revisar con las reglas de esta versi\u00f3n de Qadra.',
  },
};

const MISSING = 'Falta evidencia para decidirlo';
const STATUS = {
  checked: { status: 'Correcto', tone: 'success' },
  failed: { status: 'Fallido', tone: 'danger' },
  indeterminate: { status: MISSING, tone: 'warning' },
  'not-supported': { status: 'No compatible', tone: 'neutral' },
  'not-applicable': { status: 'No aplica', tone: 'muted' },
};

const TITLES = {
  package: 'Paquete',
  'document-token': 'Sello de tiempo del documento',
  'document-signature': 'Firma del documento',
  'binding-consent': 'Vinculaci\u00f3n y consentimiento',
  certificates: 'Certificados y ra\u00edz de confianza',
  revocation: 'Revocaci\u00f3n',
  manifest: 'Manifiesto y su sello de tiempo',
  context: 'Fechas y registros',
};

const INSTANT_LABELS = {
  'document token genTime': 'Fecha del sello de tiempo del documento',
  'manifest token genTime': 'Fecha del sello de tiempo del manifiesto',
  binding_registered_at: 'Registro del certificado en Qadra',
  accepted_at: 'Aceptaci\u00f3n de la firma',
  'receipt accepted_at': 'Aceptaci\u00f3n de la firma, seg\u00fan el recibo',
  'current status': 'Momento de esta revisi\u00f3n',
};

const INSTANT_SOURCES = {
  token: 'Sello de tiempo verificado',
  'application-record': 'Registro de Qadra sin sello de tiempo',
  caller: 'Reloj de este equipo',
};

const unique = (values) => [...new Set(values)];
const list = (value) => (Array.isArray(value) ? value : []);

/** The overall result as `{ label, tone, summary }`. */
export function overallLabel(report) {
  return OVERALL[report?.overall] || OVERALL['not-supported'];
}

export function profileLabel(profile) {
  return PROFILE_LABELS[profile] || 'sin reconocer';
}

/** The messages of the findings that did not pass, without repetitions. */
export function problemMessages(findings) {
  return unique(
    list(findings)
      .filter((finding) => !['checked', 'not-applicable'].includes(finding?.outcome))
      .map((finding) => findingMessage(finding.code)),
  );
}

function strictProfile(findings) {
  const profile = findings.filter(
    (finding) => finding.code === 'cert.profile' || finding.code?.startsWith('cert.profile.'),
  );
  if (!profile.length) return null;
  return profile.some((finding) => finding.outcome !== 'checked')
    ? 'No cumple el perfil estricto'
    : 'Cumple el perfil estricto';
}

function rootTrust(findings) {
  const codes = new Set(findings.map((finding) => finding.code));
  if (codes.has('trust.root_not_pinned') || codes.has('token.anchor_not_pinned'))
    return 'La ra\u00edz no est\u00e1 en la lista fija de Qadra';
  if (codes.has('trust.root_served_not_pinned')) return 'Confianza sin confirmar';
  if (codes.has('trust.root_pinned')) return 'Ra\u00edz fija en Qadra';
  return null;
}

/**
 * One component as `{ kind, title, outcome, status, tone, messages, profile,
 * trust }`. The signature reads as correct or incorrect; the certificates
 * carry the strict profile and the trust of the root separately.
 */
export function componentView(component) {
  const findings = list(component?.findings);
  const outcome = component?.outcome;
  const base = STATUS[outcome] || STATUS['not-supported'];
  let status = base.status;
  if (component?.kind === 'document-signature' && outcome === 'checked') status = 'Firma correcta';
  if (component?.kind === 'document-signature' && outcome === 'failed') status = 'Firma incorrecta';
  const certificates = component?.kind === 'certificates';
  return {
    kind: component?.kind,
    title: TITLES[component?.kind] || 'Otra comprobaci\u00f3n',
    outcome,
    status,
    tone: base.tone,
    messages: problemMessages(findings),
    profile: certificates ? strictProfile(findings) : null,
    trust: certificates ? rootTrust(findings) : null,
  };
}

function utc(unix) {
  const date = new Date(Number(unix) * 1000);
  if (!Number.isFinite(date.getTime())) return 'Fecha no v\u00e1lida';
  return `${date.toISOString().slice(0, 19).replace('T', ' ')} UTC`;
}

/** One instant as `{ label, when, source }`, written in UTC. */
export function instantView(instant) {
  return {
    label: INSTANT_LABELS[instant?.label] || 'Otra fecha',
    when: utc(instant?.unix),
    source: INSTANT_SOURCES[instant?.source] || 'Origen sin reconocer',
  };
}

/** True when the file is a ZIP that Qadra did not write, such as a repacked export. */
export function isNotQadraExport(report) {
  const component = list(report?.components).find((item) => item?.kind === 'package');
  return (
    component?.outcome === 'not-supported' &&
    list(component.findings).some(
      (finding) => finding?.outcome === 'not-supported' && finding.code?.startsWith('zip.'),
    )
  );
}

/** The pinned roots the report shows were used to admit a root. */
export function appliedRoots(report, pinned) {
  const details = list(report?.components)
    .flatMap((component) => list(component?.findings))
    .filter((finding) => ['trust.root_pinned', 'token.anchor_pinned'].includes(finding?.code))
    .map((finding) => String(finding.detail ?? ''));
  return list(pinned).filter(({ label }) =>
    details.some(
      (detail) => detail.startsWith(`${label} for `) || detail.includes(`: ${label} for `),
    ),
  );
}

/** The trust notes of the report as plain messages, without repetitions. */
export function trustNotes(report) {
  const seen = new Set();
  const notes = [];
  for (const note of list(report?.trust)) {
    const message = findingMessage(note?.code);
    if (seen.has(message)) continue;
    seen.add(message);
    notes.push({ code: note.code, message });
  }
  return notes;
}
