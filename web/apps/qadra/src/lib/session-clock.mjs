const maximumLifetimeSeconds = 86400;
const maximumEpochMilliseconds = 253402300799999;

function integer(value, minimum, maximum) {
  return Number.isSafeInteger(value) && value >= minimum && value <= maximum;
}

function monotonic(value) {
  return typeof value === 'number' && Number.isFinite(value) && value >= 0;
}

export function confirmSessionWindow(state, { startedAt, receivedAt }) {
  const policy = state?.policy;
  const now = state?.server_now_unix_ms;
  const absolute = state?.absolute_expires_at_unix_ms;
  const idle = state?.idle_expires_at_unix_ms;
  if (
    !integer(policy?.absolute_ttl_seconds, 1, maximumLifetimeSeconds) ||
    (policy.idle_ttl_seconds !== null &&
      !integer(policy.idle_ttl_seconds, 1, policy.absolute_ttl_seconds)) ||
    !integer(now, 0, maximumEpochMilliseconds) ||
    !integer(absolute, 1, maximumEpochMilliseconds) ||
    absolute <= now ||
    absolute - now > policy.absolute_ttl_seconds * 1000 ||
    !monotonic(startedAt) ||
    !monotonic(receivedAt) ||
    receivedAt < startedAt
  )
    throw new Error('Los plazos de sesi\u00f3n recibidos no son v\u00e1lidos.');
  if (
    (policy.idle_ttl_seconds === null && idle !== null) ||
    (policy.idle_ttl_seconds !== null &&
      (!integer(idle, now + 1, absolute) || idle - now > policy.idle_ttl_seconds * 1000))
  )
    throw new Error('El plazo de inactividad no corresponde a la sesi\u00f3n.');

  // Subtract the complete round trip so transport latency cannot extend a deadline.
  const remainingMsAtReceipt = Math.max(
    0,
    Math.min(absolute, idle ?? absolute) - now - (receivedAt - startedAt),
  );
  return Object.freeze({
    serverNowMs: now,
    absoluteExpiresAtMs: absolute,
    idleExpiresAtMs: idle,
    receivedAt,
    remainingMsAtReceipt,
    expiresAtMonotonicMs: receivedAt + remainingMsAtReceipt,
  });
}

export function remainingSessionMilliseconds(window, monotonicNow) {
  if (!monotonic(monotonicNow) || monotonicNow < window.receivedAt)
    throw new Error('No se pudo confirmar el tiempo transcurrido de la sesi\u00f3n.');
  return Math.max(0, window.expiresAtMonotonicMs - monotonicNow);
}
