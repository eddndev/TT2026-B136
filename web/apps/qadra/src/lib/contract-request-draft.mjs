// Local checks of the contract creation and signature request forms. They
// repeat the limits of the HTTP contract so the form explains a problem
// before sending; the server remains the authority.
// Rationale: docs/adr/0086-bilateral-contract-signing.md.

const CONTROL = /[\u0000-\u001f\u007f-\u009f]/;
const EMAIL = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
const UUID = /^[0-9a-f]{8}(-[0-9a-f]{4}){3}-[0-9a-f]{12}$/;
export const DEFAULT_TTL_DAYS = 7;

function text(value, maximum, message) {
  if (typeof value !== 'string') throw new Error(message);
  const trimmed = value.trim();
  if (!trimmed || [...trimmed].length > maximum || CONTROL.test(trimmed)) throw new Error(message);
  return trimmed;
}

export function contractTitle(value) {
  return text(value, 160, 'El t\u00edtulo debe tener entre 1 y 160 caracteres, sin controles.');
}

export function contractReason(value, required) {
  if (!required && (value === undefined || value === null || String(value).trim() === ''))
    return null;
  return text(value, 500, 'El motivo debe tener entre 1 y 500 caracteres, sin controles.');
}

export function contractRequestValues(draft) {
  const clientEmail = typeof draft?.clientEmail === 'string' ? draft.clientEmail.trim() : '';
  if (!EMAIL.test(clientEmail) || clientEmail.length > 254)
    throw new Error('Escribe el correo exacto de la cuenta del Cliente.');
  if (typeof draft.firmSignerId !== 'string' || !UUID.test(draft.firmSignerId))
    throw new Error('Elige qui\u00e9n firma por el despacho.');
  const ttlDays = Number(draft.ttlDays);
  if (!Number.isInteger(ttlDays) || ttlDays < 1 || ttlDays > 30)
    throw new Error('La vigencia debe ser de 1 a 30 d\u00edas.');
  if (!Number.isInteger(draft.version) || draft.version < 1)
    throw new Error('Elige la versi\u00f3n exacta del documento.');
  const ceremonyReference = text(
    draft.ceremonyReference,
    80,
    'Escribe el folio de la hoja de huella firmada, de 1 a 80 caracteres.',
  );
  return {
    clientEmail,
    firmSignerId: draft.firmSignerId,
    ttlDays,
    version: draft.version,
    ceremonyReference,
  };
}
