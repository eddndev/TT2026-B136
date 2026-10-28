// Reads the trust the server currently admits for evidence, from
// GET /api/v1/evidence/trust. The answer only changes the current status
// and the trust notes of a validation, so any failure gives null and the
// validation runs with the pinned roots alone. Rationale: the section
// "Trust anchors" of docs/adr/0085-in-browser-evidence-validation.md.

const isObject = (value) => !!value && typeof value === 'object' && !Array.isArray(value);

/** The trust route response, or null when it is unavailable or malformed. */
export async function fetchEvidenceTrust(request) {
  try {
    const value = await request('/evidence/trust');
    return isObject(value) && isObject(value['server-seal']) ? value : null;
  } catch {
    return null;
  }
}
