import {
  confirmSessionWindow,
  remainingSessionMilliseconds,
} from '../../../qadra/src/lib/session-clock.mjs';

const messages = {
  invalid_credentials: 'Correo o contrase\u00f1a incorrectos.',
  mfa_rejected: 'El c\u00f3digo fue rechazado o venci\u00f3. Inicia sesi\u00f3n de nuevo.',
  account_locked: 'Demasiados intentos. Espera antes de volver a iniciar sesi\u00f3n.',
  permission_denied: 'Esta cuenta no tiene acceso como operador CA.',
  invalid_session: 'Tu sesi\u00f3n termin\u00f3. Inicia sesi\u00f3n de nuevo.',
  invalid_response: 'No se pudo confirmar la respuesta del servidor. Inicia sesi\u00f3n de nuevo.',
  challenge_required: 'Inicia sesi\u00f3n de nuevo para obtener otro desaf\u00edo.',
  invalid_factor: 'Selecciona un m\u00e9todo de verificaci\u00f3n v\u00e1lido.',
  auth_attempt_superseded: 'Este intento de acceso ya no est\u00e1 activo.',
  network_error: 'No se pudo conectar con el servicio CA. Vuelve a intentarlo.',
  service_error: 'El servicio CA no pudo completar la solicitud. Vuelve a intentarlo.',
};

export function failure(code, status) {
  const safeCode = Object.hasOwn(messages, code) ? code : 'service_error';
  const error = new Error(messages[safeCode]);
  error.code = safeCode;
  if (status !== undefined) error.status = status;
  return error;
}

export function credential(value) {
  return typeof value === 'string' && /^[\x21-\x7e]{1,4096}$/.test(value);
}

export function readSession(value, receipt, previous = null) {
  const operator = value?.operator;
  if (
    typeof operator?.id !== 'string' ||
    !/^[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}$/i.test(operator.id) ||
    typeof operator.email !== 'string' || !operator.email.trim() ||
    operator.email.length > 254 || operator.role !== undefined ||
    value.user !== undefined || value.role !== undefined
  ) throw failure('invalid_response');
  let window;
  try {
    window = confirmSessionWindow(value, receipt);
  } catch {
    throw failure('invalid_response');
  }
  if (previous && (
    operator.id !== previous.operator.id || operator.email !== previous.operator.email ||
    value.policy.absolute_ttl_seconds !== previous.policy.absolute_ttl_seconds ||
    value.policy.idle_ttl_seconds !== previous.policy.idle_ttl_seconds ||
    window.absoluteExpiresAtMs !== previous.window.absoluteExpiresAtMs ||
    window.serverNowMs < previous.window.serverNowMs
  )) throw failure('invalid_response');
  const offset = Math.min(
    previous?.offset ?? Infinity,
    receipt.startedAt - window.serverNowMs,
  );
  window = Object.freeze({
    ...window,
    expiresAtMonotonicMs:
      Math.min(window.absoluteExpiresAtMs, window.idleExpiresAtMs ?? Infinity) + offset,
  });
  if (remainingSessionMilliseconds(window, receipt.receivedAt) <= 0)
    throw failure('invalid_session');
  return {
    operator: Object.freeze({ id: operator.id, email: operator.email }),
    policy: Object.freeze({ ...value.policy }),
    window,
    offset,
  };
}

export function remaining(session, now) {
  return session ? remainingSessionMilliseconds(session.window, now) : 0;
}
