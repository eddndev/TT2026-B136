import { credential, failure, readSession, remaining } from './ca-access-values.mjs';

export function createCaAccessClient({
  fetcher = globalThis.fetch,
  now = () => performance.now(),
  setTimer = globalThis.setTimeout,
  clearTimer = globalThis.clearTimeout,
  onState = () => {},
} = {}) {
  let phase = 'signed-out';
  let generation = 0;
  let challenge = null;
  let bearer = '';
  let session = null;
  let timer = null;
  let controlFlight = null;
  let disposed = false;

  function view() {
    let remainingMs = 0;
    try { remainingMs = remaining(session, now()); } catch { /* Admission checks expire it. */ }
    return Object.freeze({ phase, operator: session?.operator ?? null, remainingMs });
  }

  function publish() { onState(view()); }

  function stopTimer() {
    if (timer !== null) clearTimer(timer);
    timer = null;
  }

  function close(next = 'signed-out') {
    generation++;
    stopTimer();
    phase = next;
    challenge = session = controlFlight = null;
    bearer = '';
    publish();
  }

  function checkTime() {
    if (phase === 'active') {
      try {
        if (remaining(session, now()) <= 0) close('expired');
      } catch { close('expired'); }
    } else if (challenge && now() >= challenge.until) close();
  }

  function snapshot() {
    checkTime();
    return view();
  }

  function arm() {
    stopTimer();
    checkTime();
    const delay = phase === 'active' ? remaining(session, now()) : challenge?.until - now();
    if (!Number.isFinite(delay) || delay <= 0) return;
    const expected = generation;
    timer = setTimer(() => {
      if (generation !== expected) return;
      timer = null;
      checkTime();
      arm();
    }, delay);
  }

  function current(expected) {
    if (disposed || generation !== expected) throw failure('auth_attempt_superseded');
  }

  function active() {
    checkTime();
    if (disposed || phase !== 'active' || !bearer) throw failure('invalid_session');
  }

  async function request(path, { method = 'GET', data, token = '' } = {}, scope = 'auth') {
    const headers = {};
    if (token) headers.Authorization = `Bearer ${token}`;
    if (data !== undefined) headers['Content-Type'] = 'application/json';
    let response;
    try {
      response = await fetcher(`/api/v1/${scope}/${path}`, {
        method, headers,
        body: data === undefined ? undefined : JSON.stringify(data),
        cache: 'no-store', credentials: 'omit', redirect: 'error',
      });
    } catch { throw failure('network_error'); }
    if (!response.ok) {
      const value = await response.json().catch(() => null);
      throw failure(value?.error?.code, response.status);
    }
    if (response.status === 204) return null;
    try { return await response.json(); } catch { throw failure('invalid_response'); }
  }

  async function login(email, password) {
    if (disposed) throw failure('auth_attempt_superseded');
    close();
    const expected = generation;
    const startedAt = now();
    try {
      const value = await request('login', { method: 'POST', data: { email, password } });
      current(expected);
      if (!credential(value?.challenge_token) ||
          !Number.isSafeInteger(value.expires_in_seconds) ||
          value.expires_in_seconds < 1 || value.expires_in_seconds > 300)
        throw failure('invalid_response');
      const until = startedAt + value.expires_in_seconds * 1000;
      if (now() >= until) throw failure('challenge_required');
      challenge = { token: value.challenge_token, until, email: email.trim().toLowerCase() };
      phase = 'mfa';
      arm();
      publish();
      return snapshot();
    } catch (error) {
      current(expected);
      close();
      throw error;
    }
  }

  async function mfa(code, mode = 'totp') {
    checkTime();
    if (!['totp', 'recovery'].includes(mode)) throw failure('invalid_factor');
    if (disposed || phase !== 'mfa' || !challenge) throw failure('challenge_required');
    const captured = challenge;
    challenge = null;
    stopTimer();
    const expected = ++generation;
    const startedAt = now();
    try {
      const value = await request(`mfa/${mode}`, {
        method: 'POST', data: { challenge_token: captured.token, code },
      });
      current(expected);
      if (!credential(value?.access_token) || value.token_type !== 'Bearer' ||
          !Number.isSafeInteger(value.expires_in_seconds) || value.expires_in_seconds < 1)
        throw failure('invalid_response');
      const confirmed = readSession(value, { startedAt, receivedAt: now() });
      if (confirmed.operator.email !== captured.email) throw failure('invalid_response');
      session = confirmed;
      bearer = value.access_token;
      phase = 'active';
      arm();
      publish();
      return snapshot();
    } catch (error) {
      current(expected);
      close();
      throw error;
    }
  }

  async function control(path, method) {
    active();
    const expected = generation;
    if (controlFlight) {
      await controlFlight.promise;
      current(expected);
      return control(path, method);
    }
    const original = session;
    const startedAt = now();
    const flight = {};
    controlFlight = flight;
    flight.promise = (async () => {
      try {
        const value = await request(path, { method, token: bearer });
        current(expected);
        active();
        session = readSession(value, { startedAt, receivedAt: now() }, original);
        arm();
        publish();
        return snapshot();
      } catch (error) {
        current(expected);
        if ([401, 403].includes(error.status) ||
            ['invalid_response', 'invalid_session'].includes(error.code)) close('expired');
        throw error;
      } finally {
        if (controlFlight === flight) controlFlight = null;
      }
    })();
    return flight.promise;
  }

  async function logout() {
    checkTime();
    const token = bearer;
    close();
    if (token) await request('logout', { method: 'POST', token });
    return snapshot();
  }

  async function command(data) {
    active();
    const expected = generation;
    try {
      const value = await request('operations', { method: 'POST', data, token: bearer }, 'ca');
      current(expected);
      active();
      return value;
    } catch (error) {
      current(expected);
      if ([401, 403].includes(error.status)) close('expired');
      throw error;
    }
  }

  publish();
  return {
    login, mfa, snapshot, logout, command,
    sessionStatus: () => control('session', 'GET'),
    recordActivity: () => control('activity', 'POST'),
    cancelAuthentication: () => close(),
    dispose() { disposed = true; close(); },
  };
}
