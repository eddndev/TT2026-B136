// Human display name of an office account, written by the Owner when the account
// is created: docs/adr/0087-account-display-names.md. The server is authoritative;
// this mirror rejects the same input before it is sent. Control, format (Cf) and
// line or paragraph separator characters are refused before trimming, and the
// trimmed value must hold 1..=120 Unicode scalars. The Cf class follows the
// Unicode version of the JavaScript engine, which may be newer than the server's.
export const accountNameMax = 120;
const refused = /[\p{Cc}\p{Cf}\u2028\u2029]/u;
function invalid() {
  throw new Error(
    'Escribe un nombre de 1 a 120 caracteres, sin saltos de l\u00ednea ni caracteres de control.',
  );
}
export function accountName(value) {
  if (typeof value !== 'string' || refused.test(value)) invalid();
  const trimmed = value.trim();
  const length = [...trimmed].length;
  if (length < 1 || length > accountNameMax) invalid();
  return trimmed;
}
// True when a received name is already a valid, trimmed account name.
export function canonicalAccountName(value) {
  try {
    return accountName(value) === value;
  } catch {
    return false;
  }
}
// Up to two initials from the first two words of the name, or the first two
// letters of the email when the account has no name.
export function accountInitials(user) {
  const words = typeof user?.name === 'string' ? user.name.trim().split(/\s+/).filter(Boolean) : [];
  if (!words.length)
    return String(user?.email ?? '')
      .slice(0, 2)
      .toUpperCase();
  return words
    .slice(0, 2)
    .map((word) => String.fromCodePoint(word.codePointAt(0)).toLocaleUpperCase('es-MX'))
    .join('');
}
