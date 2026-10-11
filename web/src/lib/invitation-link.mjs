const prefix = '#invitation=';
export function takeInvitationLink(browser = globalThis) {
  const hash = browser.location.hash;
  if (hash !== '#invitation' && !hash.startsWith(prefix)) return null;
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
  if (!/^[A-Za-z0-9_-]{43}$/.test(token)) return { status: 'invalid' };
  try {
    const bytes = atob(token.replaceAll('-', '+').replaceAll('_', '/') + '=');
    const encoded = btoa(bytes).replaceAll('+', '-').replaceAll('/', '_').replace(/=+$/, '');
    return bytes.length === 32 && encoded === token
      ? { status: 'ready', token }
      : { status: 'invalid' };
  } catch {
    return { status: 'invalid' };
  }
}
