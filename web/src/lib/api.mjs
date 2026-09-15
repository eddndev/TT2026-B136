import { caseApi } from './case-api.mjs';

const messages = {
  invalid_credentials: 'Correo o contrase\u00f1a incorrectos.',
  mfa_rejected: 'El c\u00f3digo fue rechazado o venci\u00f3. Vuelve a iniciar sesi\u00f3n.',
  invalid_session: 'Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.',
  permission_denied: 'No tienes permiso para realizar esta acci\u00f3n.',
  account_locked: 'Demasiados intentos. Espera 15 minutos antes de volver a intentarlo.',
  document_not_found: 'No se encontr\u00f3 un documento con ese identificador.',
  document_version_required:
    'Selecciona una versi\u00f3n del historial para realizar esta acci\u00f3n.',
  document_version_exhausted:
    'Este documento ha alcanzado el l\u00edmite de versiones. Conserva tu archivo y c\u00e1rgalo como un documento nuevo.',
  document_already_sealed: 'Este documento ya est\u00e1 sellado.',
  document_not_sealed: 'Primero sella el documento para verificarlo o descargar su evidencia.',
  user_already_exists:
    'Ya existe una cuenta con ese correo. Usa otro correo para el nuevo integrante.',
  bootstrap_closed: 'El despacho ya tiene un administrador. Inicia sesi\u00f3n con tu cuenta.',
  invalid_input: 'Revisa los datos ingresados y los l\u00edmites de cada campo.',
  case_not_found: 'El expediente no est\u00e1 disponible o ya no tienes acceso.',
  user_not_found: 'No se encontr\u00f3 un usuario activo con ese identificador.',
  invalid_document_name: 'El nombre del documento no es v\u00e1lido.',
};

export function createApi(fetcher = globalThis.fetch, onExpired = () => {}) {
  let token = '';
  let sessionVersion = 0;
  async function request(
    path,
    { method = 'GET', data, body, headers = {}, protectedRoute = true, binary = false } = {},
  ) {
    const requestVersion = sessionVersion;
    const assertCurrentSession = () => {
      if (protectedRoute && requestVersion !== sessionVersion) {
        throw new Error('La sesi\u00f3n de esta solicitud termin\u00f3.');
      }
    };
    const requestHeaders = { ...headers };
    if (protectedRoute && token) requestHeaders.Authorization = `Bearer ${token}`;
    if (data !== undefined) requestHeaders['Content-Type'] = 'application/json';
    let response;
    try {
      response = await fetcher(`/api/v1${path}`, {
        method,
        headers: requestHeaders,
        body: data === undefined ? body : JSON.stringify(data),
        cache: 'no-store',
        credentials: 'omit',
        redirect: 'error',
      });
    } catch {
      throw new Error('No se pudo conectar con el servidor. Comprueba que est\u00e9 disponible.');
    }
    assertCurrentSession();
    if (!response.ok) {
      const payload = await response.json().catch(() => ({}));
      assertCurrentSession();
      if (response.status === 401 && protectedRoute) {
        token = '';
        sessionVersion++;
        onExpired();
      }
      const fallback =
        response.status === 409
          ? 'No se pudo completar la operaci\u00f3n con el estado actual. Revisa los datos e int\u00e9ntalo de nuevo.'
          : response.status === 413
            ? 'El documento supera el l\u00edmite de 16 MiB.'
            : response.status >= 500
              ? 'El servidor no pudo completar la operaci\u00f3n. Vuelve a intentarlo m\u00e1s tarde.'
              : `No se pudo completar la operaci\u00f3n (HTTP ${response.status}).`;
      const error = new Error(messages[payload.error?.code] || fallback);
      error.status = response.status;
      error.code = payload.error?.code;
      throw error;
    }
    const result = binary
      ? {
          blob: await response.blob(),
          digest: response.headers.get('X-Document-Digest'),
          documentId: response.headers.get('X-Document-Id'),
          version: response.headers.get('X-Document-Version'),
        }
      : response.status === 204
        ? null
        : await response.json();
    assertCurrentSession();
    return result;
  }
  const post = (path, data, protectedRoute = true) =>
    request(path, { method: 'POST', data, protectedRoute });
  return {
    login: (email, password) => post('/auth/login', { email, password }, false),
    bootstrap: (email, password) => post('/auth/bootstrap', { email, password }, false),
    async mfa(challenge_token, code, mode) {
      if (!['totp', 'recovery'].includes(mode))
        throw new Error('M\u00e9todo de verificaci\u00f3n no v\u00e1lido.');
      const session = await post(`/auth/mfa/${mode}`, { challenge_token, code }, false);
      token = session.access_token;
      sessionVersion++;
      return session;
    },
    me: () => request('/auth/me'),
    async logout() {
      await post('/auth/logout');
      token = '';
      sessionVersion++;
    },
    createUser: (email, password, role) => post('/users', { email, password, role }),
    ...caseApi(request),
    audit: () => request('/audit/verify'),
  };
}
