// The pinned roots built into a Qadra release: the SHA-256 fingerprints of
// the root certificates the evidence validator admits, each with the
// purposes it anchors. A root inside an evidence package is trusted only when
// it is listed here for the purpose it serves. Builds for other environments
// replace the file through QADRA_PINNED_ROOTS. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.

export const PURPOSES = Object.freeze(['server-seal', 'individual-signing', 'time-stamping']);
const FINGERPRINT = /^[0-9a-f]{64}$/;
const KEYS = ['fingerprint', 'label', 'purposes'];

function fail(reason) {
  throw new Error(`Invalid pinned roots: ${reason}.`);
}

function root(value, index) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`entry ${index}`);
  const keys = Object.keys(value).sort();
  if (keys.length !== KEYS.length || keys.some((key, at) => key !== KEYS[at]))
    fail(`entry ${index} must have exactly fingerprint, purposes and label`);
  const { fingerprint, purposes, label } = value;
  if (typeof fingerprint !== 'string' || !FINGERPRINT.test(fingerprint))
    fail(`entry ${index} fingerprint must be 64 lowercase hexadecimal digits`);
  if (!Array.isArray(purposes) || !purposes.length) fail(`entry ${index} has no purpose`);
  if (purposes.some((purpose) => !PURPOSES.includes(purpose)))
    fail(`entry ${index} names an unknown purpose`);
  if (new Set(purposes).size !== purposes.length) fail(`entry ${index} repeats a purpose`);
  if (typeof label !== 'string' || !label.trim() || label.length > 200)
    fail(`entry ${index} needs a label of at most 200 characters`);
  return Object.freeze({
    fingerprint,
    purposes: Object.freeze(PURPOSES.filter((purpose) => purposes.includes(purpose))),
    label,
  });
}

/** Validate a parsed pinned roots document and return a frozen copy. */
export function parsePinnedRoots(value) {
  if (!Array.isArray(value) || !value.length) fail('expected a non-empty list');
  const roots = value.map(root);
  if (new Set(roots.map((entry) => entry.fingerprint)).size !== roots.length)
    fail('a fingerprint is listed twice');
  return Object.freeze(roots);
}

/**
 * The pinned roots file a build uses: `override` when set, which must be an
 * absolute path, or `defaultFile`. `read` returns a file's text; the chosen
 * file is validated so that a bad override fails the build.
 */
export function pinnedRootsSource(override, defaultFile, read) {
  if (override && !override.startsWith('/'))
    throw new Error('QADRA_PINNED_ROOTS must be an absolute path.');
  const file = override || defaultFile;
  let value;
  try {
    value = JSON.parse(read(file));
  } catch {
    fail(`${file} is not readable JSON`);
  }
  parsePinnedRoots(value);
  return file;
}
