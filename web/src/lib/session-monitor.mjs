import { confirmSessionWindow, remainingSessionMilliseconds } from './session-clock.mjs';

export function createSessionMonitor({
  api,
  now = () => performance.now(),
  setTimer = globalThis.setTimeout,
  clearTimer = globalThis.clearTimeout,
  onExpired = () => {},
  onError = () => {},
  activityIntervalMs = 10000,
}) {
  if (!Number.isFinite(activityIntervalMs) || activityIntervalMs <= 0)
    throw new Error('El intervalo de actividad debe ser positivo.');
  let generation = 0;
  let state = null;
  let timer = null;
  let pending = null;
  let lastActivityAttempt = -Infinity;

  function stop() {
    generation++;
    if (timer !== null) clearTimer(timer);
    timer = null;
    state = null;
    pending = null;
    lastActivityAttempt = -Infinity;
  }

  function expire(reason) {
    if (!state) return;
    stop();
    onExpired(reason);
  }

  function remaining() {
    if (!state) return 0;
    try {
      return remainingSessionMilliseconds(state.window, now());
    } catch {
      expire('invalid-state');
      return 0;
    }
  }

  function active() {
    if (state && remaining() <= 0) expire('deadline');
    return state !== null;
  }

  function arm() {
    if (timer !== null) clearTimer(timer);
    timer = null;
    if (!active()) return;
    const expected = generation;
    timer = setTimer(() => {
      if (expected === generation) arm();
    }, remaining());
  }

  function start(value, receipt) {
    stop();
    const window = confirmSessionWindow(value, receipt);
    if (typeof value?.user?.id !== 'string' || !value.user.id)
      throw new Error('La sesi\u00f3n recibida no tiene una cuenta v\u00e1lida.');
    state = {
      user: { id: value.user.id, email: value.user.email, role: value.user.role },
      policy: { ...value.policy },
      window,
      serverToMonotonicMs: receipt.startedAt - window.serverNowMs,
    };
    arm();
  }

  async function request(kind) {
    if (!active() || pending) return false;
    const expected = generation;
    const original = state;
    const startedAt = now();
    const operation = {};
    pending = operation;
    operation.promise = (async () => {
      try {
        const value = await (kind === 'activity' ? api.recordActivity() : api.sessionStatus());
        if (generation !== expected || !active()) return false;
        let window;
        try {
          window = confirmSessionWindow(value, { startedAt, receivedAt: now() });
        } catch {
          expire('invalid-state');
          return false;
        }
        if (
          value.user?.id !== original.user.id ||
          value.user?.email !== original.user.email ||
          value.user?.role !== original.user.role ||
          value.policy.absolute_ttl_seconds !== original.policy.absolute_ttl_seconds ||
          value.policy.idle_ttl_seconds !== original.policy.idle_ttl_seconds ||
          window.absoluteExpiresAtMs !== original.window.absoluteExpiresAtMs ||
          window.serverNowMs < original.window.serverNowMs
        ) {
          expire('invalid-state');
          return false;
        }
        // Later, faster replies cannot return time already deducted for transport.
        const serverToMonotonicMs = Math.min(
          original.serverToMonotonicMs,
          startedAt - window.serverNowMs,
        );
        window = Object.freeze({
          ...window,
          expiresAtMonotonicMs:
            Math.min(window.absoluteExpiresAtMs, window.idleExpiresAtMs ?? Infinity) +
            serverToMonotonicMs,
        });
        state = { ...original, window, serverToMonotonicMs };
        arm();
        return active();
      } catch (error) {
        if (generation !== expected || !active()) return false;
        if (error.status === 401) expire('rejected');
        else onError(error);
        return false;
      } finally {
        if (pending === operation) pending = null;
      }
    })();
    return operation.promise;
  }

  return {
    start,
    stop,
    active,
    remaining,
    refresh: () => pending?.promise ?? request('read'),
    activity() {
      if (!active() || pending || state.policy.idle_ttl_seconds === null)
        return Promise.resolve(false);
      const interval = Math.min(activityIntervalMs, state.policy.idle_ttl_seconds * 250);
      if (now() - lastActivityAttempt < interval) return Promise.resolve(false);
      lastActivityAttempt = now();
      return request('activity');
    },
  };
}
