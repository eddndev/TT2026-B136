// The policy the evidence validation worker passes to the engine: the pinned
// roots, the trust the server currently serves and the instant of the current
// status. The served trust only changes the current status and the trust
// notes; the historical result depends on the pinned roots alone. A served
// response with any value that is not canonical padded base64 is treated as
// unavailable, so that route data can never make the engine reject the whole
// policy. Rationale: docs/adr/0085-in-browser-evidence-validation.md.
import { PURPOSES } from './pinned-roots-schema.mjs';

/** The time-stamp policy of the internal authority. */
export const DEFAULT_ADMITTED_POLICY = '1.3.6.1.4.1.13762.3';

const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;
const MALFORMED = Symbol('malformed');

/** Canonical padded base64, as the engine decodes it, or MALFORMED. */
function base64(value) {
  if (typeof value !== 'string' || !value || !BASE64.test(value)) return MALFORMED;
  try {
    return btoa(atob(value)) === value ? value : MALFORMED;
  } catch {
    return MALFORMED;
  }
}

/** An optional value: absent gives null, anything else must be base64. */
const optional = (value) => (value === undefined || value === null ? null : base64(value));

function servedTrust(trust) {
  const seal = trust?.['server-seal'];
  if (!seal || typeof seal !== 'object' || base64(seal.root_der_base64) === MALFORMED) return null;
  const individual = trust['individual-signing'];
  const found = [[seal.root_der_base64, 'server-seal']];
  const stamping = trust['time-stamping']?.roots;
  for (const root of Array.isArray(stamping) ? stamping : [])
    found.push([optional(root?.root_der_base64), 'time-stamping']);
  found.push([optional(individual?.root_der_base64), 'individual-signing']);
  const crls = [optional(seal.crl_der_base64), optional(individual?.crl_der_base64)];
  if ([...found.map(([der]) => der), ...crls].includes(MALFORMED)) return null;
  const merged = new Map();
  for (const [der, purpose] of found) {
    if (der === null) continue;
    if (!merged.has(der)) merged.set(der, new Set());
    merged.get(der).add(purpose);
  }
  const roots = [...merged].map(([der, purposes]) => ({
    der_base64: der,
    purposes: PURPOSES.filter((purpose) => purposes.has(purpose)),
  }));
  return { roots, crls_base64: crls.filter((crl) => crl !== null) };
}

/**
 * Build the validation policy from the pinned roots, the trust route
 * response (or null when the route failed) and `now`, a Date or a number of
 * milliseconds since the Unix epoch read from the client clock.
 */
export function buildPolicy({ pinned, trust, now }) {
  const milliseconds = now instanceof Date ? now.getTime() : now;
  if (!Number.isFinite(milliseconds)) throw new TypeError('now');
  return {
    pinned: pinned.map(({ fingerprint, purposes, label }) => ({
      fingerprint,
      purposes: [...purposes],
      label,
    })),
    served: servedTrust(trust),
    admitted_policies: [DEFAULT_ADMITTED_POLICY],
    current_instant: Math.floor(milliseconds / 1000),
  };
}
