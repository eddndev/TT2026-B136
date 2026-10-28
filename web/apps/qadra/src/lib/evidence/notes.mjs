// The notes that frame a validation result: what a correct document
// signature means when the overall result is not complete, the informational
// current status of each certificate and the comparison of the pinned roots
// with the trust the server currently serves. Rationale: the sections
// "Component results and the overall result" and "Trust anchors" of
// docs/adr/0085-in-browser-evidence-validation.md.
import { findingMessage } from './messages.mjs';
import { PURPOSE_LABELS } from './presentation.mjs';

const list = (value) => (Array.isArray(value) ? value : []);

const SIGNATURE_NOTE =
  'La firma del documento es correcta: corresponde al documento incluido. Como no todas las dem\u00e1s comprobaciones pasaron, no se puede confirmar qui\u00e9n lo firm\u00f3 ni cu\u00e1ndo. Que la validaci\u00f3n no est\u00e9 completa no significa por s\u00ed solo que el documento se haya modificado.';

/**
 * A note for a correct document signature in a result that is not complete,
 * or null. A correct signature alone neither proves who signed nor shows
 * that the document was altered.
 */
export function signatureNote(report) {
  if (!report || report.overall === 'checked') return null;
  const signature = list(report.components).find((item) => item?.kind === 'document-signature');
  return signature?.outcome === 'checked' ? SIGNATURE_NOTE : null;
}

// The certificate roles the engine names in the current status details.
const SUBJECTS = {
  'server seal signer': 'Certificado del sello del servidor',
  'server seal root': 'Ra\u00edz del sello del servidor',
  holder: 'Certificado de quien firm\u00f3',
  'holder root': 'Ra\u00edz de la firma individual',
  'time-stamping certificate': 'Certificado de la autoridad del sello de tiempo',
  'time-stamping root': 'Ra\u00edz de la autoridad del sello de tiempo',
};

// The current status reads at the present, not at the instant of the
// signature, so these codes have their own wording here.
const CURRENT = {
  'current.within_validity': 'Sigue vigente hoy.',
  'current.outside_validity': 'Ya no est\u00e1 vigente hoy.',
  'crl.not_revoked': 'No est\u00e1 revocado hoy.',
  'crl.revoked': 'Est\u00e1 revocado hoy.',
  'crl.revoked_later': 'Tiene una revocaci\u00f3n con fecha posterior a hoy.',
  'crl.not_covering': 'No hay una lista de revocaci\u00f3n vigente para hoy.',
  'current.no_crl': 'No hay una lista de revocaci\u00f3n de quien la emiti\u00f3.',
};

const TONES = {
  checked: 'success',
  failed: 'danger',
  indeterminate: 'warning',
};

/** The role a current status detail starts with, as a plain label. */
function subject(detail) {
  const role = /^[^:(]*/.exec(String(detail ?? ''))[0].trim();
  return Object.hasOwn(SUBJECTS, role) ? SUBJECTS[role] : null;
}

/**
 * The current status as `{ subject, message, tone }` rows, without
 * repetitions; `subject` names the certificate, or is null for a row about
 * the whole status.
 */
export function currentStatusView(report) {
  const seen = new Set();
  const rows = [];
  for (const finding of list(report?.current_status)) {
    const code = String(finding?.code ?? '');
    // The panel already says that the current status reads this device's
    // clock, so that finding adds no row.
    if (code === 'current.instant') continue;
    const named = Object.hasOwn(CURRENT, code) ? subject(finding.detail) : null;
    const message = named ? CURRENT[code] : findingMessage(code);
    const key = `${named}|${message}`;
    if (seen.has(key)) continue;
    seen.add(key);
    rows.push({ subject: named, message, tone: TONES[finding.outcome] || 'neutral' });
  }
  return rows;
}

const AGREE =
  'Las ra\u00edces fijas de esta versi\u00f3n de Qadra coinciden con las que usa el servidor.';

// The trust notes that compare the pinned roots with the trust route; a
// token anchor outside the pinned set is reported with the document token.
const ROUTE_NOTES = [
  'trust.route_unavailable',
  'trust.pinned_not_served',
  'trust.served_not_pinned',
];

// A root named by the first eight hexadecimal digits of its fingerprint
// and the purposes the notes list for it.
function rootName(fingerprint, purposes) {
  const labels = purposes.map((purpose) => PURPOSE_LABELS[purpose] || purpose);
  const named = `con huella ${String(fingerprint).slice(0, 8)}`;
  return labels.length ? `${named} (${labels.join(', ')})` : named;
}

const COMPARISON = {
  'trust.pinned_not_served': (root) =>
    `La ra\u00edz fija ${root} ya no aparece en la confianza que usa el servidor.`,
  'trust.served_not_pinned': (root) =>
    `El servidor usa la ra\u00edz ${root}, que no est\u00e1 en la lista fija de Qadra.`,
};

/**
 * The differences between the pinned roots and the served trust as plain
 * messages, one per root and difference, or one message saying that they
 * agree.
 */
export function serverComparison(report) {
  const notes = list(report?.trust).filter((note) => ROUTE_NOTES.includes(note?.code));
  const roots = new Map();
  const messages = [];
  for (const note of notes) {
    if (!COMPARISON[note.code]) {
      messages.push(findingMessage(note.code));
      continue;
    }
    const key = `${note.code}|${note.fingerprint}`;
    if (!roots.has(key)) roots.set(key, { note, purposes: [] });
    const purposes = roots.get(key).purposes;
    if (note.purpose && !purposes.includes(note.purpose)) purposes.push(note.purpose);
  }
  for (const { note, purposes } of roots.values())
    messages.push(COMPARISON[note.code](rootName(note.fingerprint, purposes)));
  const unique = [...new Set(messages)];
  return unique.length ? unique : [AGREE];
}
