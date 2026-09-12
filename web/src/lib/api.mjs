const messages = {
  invalid_credentials: 'Correo o contrasena incorrectos.',
  mfa_rejected: 'El codigo fue rechazado o vencio. Inicia sesion otra vez.',
  invalid_session: 'Tu sesion termino. Vuelve a iniciar sesion.',
  permission_denied: 'No tienes permiso para realizar esta accion.',
  account_locked: 'Demasiados intentos. Espera 15 minutos antes de volver a intentar.',
  document_not_found: 'No se encontro un documento con ese identificador.',
  document_already_sealed: 'Este documento ya esta sellado.',
  document_not_sealed: 'Primero sella el documento para verificarlo o descargar su evidencia.',
  user_already_exists:
    'Ya existe una cuenta con ese correo. Usa otro correo para el nuevo integrante.',
  bootstrap_closed: 'El despacho ya tiene un administrador. Usa el inicio de sesion.',
  invalid_input: 'Revisa los datos ingresados y los limites de cada campo.',
  invalid_document_name: 'El nombre del documento no es valido.',
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
        throw new Error('La sesion de esta solicitud termino.');
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
      throw new Error('No se pudo conectar con la API. Comprueba que el servidor este disponible.');
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
          ? 'La operacion entra en conflicto con el estado actual. Actualiza o revisa los datos.'
          : response.status === 413
            ? 'El documento supera el limite de 16 MiB.'
            : response.status >= 500
              ? 'El servidor no pudo completar la operacion. Intenta mas tarde.'
              : `No se pudo completar la operacion (HTTP ${response.status}).`;
      const error = new Error(messages[payload.error?.code] || fallback);
      error.status = response.status;
      error.code = payload.error?.code;
      throw error;
    }
    const result = binary
      ? { blob: await response.blob(), digest: response.headers.get('X-Document-Digest') }
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
      if (!['totp', 'recovery'].includes(mode)) throw new Error('Metodo MFA no valido.');
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
    upload: (file, name) =>
      request('/documents', {
        method: 'POST',
        body: file,
        headers: { 'X-Document-Name': name, 'Content-Type': 'application/octet-stream' },
      }),
    seal: (id) => post(`/documents/${encodeURIComponent(id)}/seal`),
    verify: (id) => post(`/documents/${encodeURIComponent(id)}/verify`),
    evidence: (id) => request(`/documents/${encodeURIComponent(id)}/evidence`, { binary: true }),
    audit: () => request('/audit/verify'),
  };
}
