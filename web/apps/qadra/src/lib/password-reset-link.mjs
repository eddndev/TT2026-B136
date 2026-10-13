const prefix = '#password-reset=';

function canonical(token) {
  if (!/^[A-Za-z0-9_-]{43}$/.test(token)) return false;
  try {
    const bytes = atob(token.replaceAll('-', '+').replaceAll('_', '/') + '=');
    return (
      bytes.length === 32 &&
      btoa(bytes).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '') === token
    );
  } catch {
    return false;
  }
}

// Remove recovery material before any caller can use it or mount a form.
// This helper neither caches the token nor writes browser storage.
export function takePasswordResetLink(browser = globalThis) {
  const hash = browser.location.hash;
  if (hash !== '#password-reset' && !hash.startsWith(prefix)) return null;
  try {
    browser.history.replaceState(
      browser.history.state,
      '',
      browser.location.pathname + browser.location.search,
    );
  } catch {
    return { status: 'unavailable' };
  }
  const token = hash.startsWith(prefix) ? hash.slice(prefix.length) : '';
  return canonical(token) ? { status: 'ready', token } : { status: 'invalid' };
}
