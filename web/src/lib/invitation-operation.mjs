const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const phases = new Set(['new', 'prepared', 'finishing']);
// Only safe operation metadata persists. Reopen the original email link to resume.
export async function invitationOperations(token, storage, crypto = globalThis.crypto) {
  const hash = await crypto.subtle.digest(
    'SHA-256',
    new TextEncoder().encode(`qadra:invitation-ui:v1\0${token}`),
  );
  const key =
    'qadra-invitation:' +
    [...new Uint8Array(hash)].map((n) => n.toString(16).padStart(2, '0')).join('');
  let state = { prepare: crypto.randomUUID(), finish: crypto.randomUUID(), phase: 'new' };
  let durable = false;
  try {
    const saved = JSON.parse(storage?.getItem(key) ?? 'null');
    if (saved && uuid.test(saved.prepare) && uuid.test(saved.finish) && phases.has(saved.phase)) {
      state = { prepare: saved.prepare, finish: saved.finish, phase: saved.phase };
    }
    storage.setItem(key, JSON.stringify(state));
    durable = true;
  } catch {
    /* A disabled store does not gain access to the invitation secret. */
  }
  return {
    get prepare() {
      return state.prepare;
    },
    get finish() {
      return state.finish;
    },
    get phase() {
      return state.phase;
    },
    get durable() {
      return durable;
    },
    mark(phase) {
      if (!phases.has(phase)) throw new TypeError('Invalid invitation phase');
      state.phase = phase;
      try {
        storage?.setItem(key, JSON.stringify(state));
      } catch {
        durable = false;
      }
    },
    clear() {
      try {
        storage?.removeItem(key);
      } catch {
        /* No credential is stored. */
      }
    },
  };
}
