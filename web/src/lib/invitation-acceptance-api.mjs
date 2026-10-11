function failure(code, message) {
  return Object.assign(new Error(message), { code });
}
// Public acceptance never attaches a session, persists credentials or retries a POST.
export function createInvitationAcceptanceApi(fetcher = globalThis.fetch) {
  let active = true;
  const ensureActive = () => {
    if (!active) throw failure('invitation_closed', 'Este registro ya no est\u00e1 abierto.');
  };
  async function post(action, data) {
    ensureActive();
    let response;
    try {
      response = await fetcher(`/api/v1/auth/invitations/${action}`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(data),
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
        referrerPolicy: 'no-referrer',
      });
    } catch {
      ensureActive();
      throw failure(
        'invitation_uncertain',
        'No se confirm\u00f3 la respuesta. Conserva el enlace y comprueba el resultado.',
      );
    }
    ensureActive();
    if (response.status === 429)
      throw failure('invitation_limited', 'Demasiados intentos. Espera antes de continuar.');
    if ([400, 401, 403, 404, 409, 422].includes(response.status)) {
      throw failure(
        'invitation_rejected',
        'El enlace o los datos no se aceptaron. Revisa la contrase\u00f1a y el c\u00f3digo; si el enlace venci\u00f3, solicita su reenv\u00edo.',
      );
    }
    if (!response.ok)
      throw failure(
        'invitation_uncertain',
        'El servicio no confirm\u00f3 el resultado. Comprueba antes de repetir.',
      );
    try {
      const value = await response.json();
      ensureActive();
      return value;
    } catch (error) {
      ensureActive();
      throw failure('invitation_uncertain', 'La respuesta no se pudo comprobar.');
    }
  }
  return {
    dispose() {
      active = false;
    },
    prepare: (token, operation_id, password) => post('prepare', { token, operation_id, password }),
    finish: (token, operation_id, password, code) =>
      post('finish', { token, operation_id, password, code }),
    receipt: (token, operation_id, password) => post('receipt', { token, operation_id, password }),
  };
}
