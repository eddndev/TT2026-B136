import { caseReportsApi } from './case-reports-api.mjs';
import { auditEventsApi } from './audit-events-api.mjs';
import { dashboardApi } from './dashboard-api.mjs';
import { caseApi } from './case-api.mjs';
import { alertsApi } from './alerts-api.mjs';
import { integrityIncidentsApi } from './document-integrity-incidents-api.mjs';
import { membersApi } from './members-api.mjs';
import { judicialCalendarsApi } from './judicial-calendars-api.mjs';
import { ownerCertificatesApi } from './owner-certificates-api.mjs';
import { ownerBase64 } from './owner-certificate-binary.mjs';
import {
  ownerLoginAvailability,
  ownerLoginChallenge,
  ownerLoginMfa,
  ownerLoginSelection,
} from './owner-login-values.mjs';

const messages = {
  audit_query_capacity_exceeded:
    'La p\u00e1gina de actividad supera su capacidad. Reduce el n\u00famero de eventos o concreta los filtros.',
  invalid_audit_query:
    'Revisa el periodo, los filtros exactos y el tama\u00f1o de la p\u00e1gina de actividad.',
  case_closed:
    'El expediente est\u00e1 cerrado administrativamente. Consulta su estado antes de modificarlo.',
  case_revision_conflict: 'Los datos del expediente cambiaron. Consulta los valores actuales.',
  case_revision_exhausted:
    'El expediente alcanz\u00f3 el l\u00edmite de revisiones. No se pueden guardar m\u00e1s cambios.',
  case_identifier_conflict:
    'El NUC o la carpeta judicial ya est\u00e1n registrados. Revisa ambos identificadores.',
  case_profile_required: 'La ficha penal completa debe conservarse. Consulta los valores actuales.',
  invalid_penal_case_profile:
    'Revisa los campos obligatorios, las descripciones y los l\u00edmites de la ficha penal.',
  invalid_case_metadata: 'Revisa el t\u00edtulo y la referencia interna.',
  case_administration_body_too_large: 'Los datos del expediente superan el l\u00edmite permitido.',

  invalid_participant_values: 'Revisa los campos del participante.',
  invalid_participant_revision: 'La revisi\u00f3n del participante no es v\u00e1lida.',
  participant_not_found: 'El participante no est\u00e1 disponible en este expediente.',
  participant_revision_exhausted:
    'El participante alcanz\u00f3 el l\u00edmite de revisiones. No se pueden guardar m\u00e1s cambios.',
  invalid_credentials: 'Correo o contrase\u00f1a incorrectos.',
  mfa_rejected: 'El c\u00f3digo fue rechazado o venci\u00f3. Vuelve a iniciar sesi\u00f3n.',
  invalid_session: 'Tu sesi\u00f3n termin\u00f3. Vuelve a iniciar sesi\u00f3n.',
  permission_denied: 'No tienes permiso para realizar esta acci\u00f3n.',
  account_locked: 'Demasiados intentos. Espera 15 minutos antes de volver a intentarlo.',
  document_not_found: 'No se encontr\u00f3 un documento con ese identificador.',
  document_format_unsupported:
    'El formato del archivo no est\u00e1 admitido. Elige un archivo de un formato compatible.',
  document_format_invalid:
    'El archivo no es v\u00e1lido o su contenido est\u00e1 da\u00f1ado. Revisa el archivo o elige otro.',
  document_validation_limit:
    'El archivo supera el l\u00edmite de validaci\u00f3n. Elige un archivo de menor complejidad o duraci\u00f3n.',
  document_validator_unavailable:
    'El validador no est\u00e1 disponible temporalmente. Conserva el archivo e int\u00e9ntalo de nuevo cuando est\u00e9 disponible.',
  document_version_required:
    'Selecciona una versi\u00f3n del historial para realizar esta acci\u00f3n.',
  document_version_exhausted:
    'Este documento ha alcanzado el l\u00edmite de versiones. Conserva tu archivo y c\u00e1rgalo como un documento nuevo.',
  document_already_sealed: 'Este documento ya est\u00e1 sellado.',
  document_not_sealed: 'Primero sella el documento para verificarlo o descargar su evidencia.',
  document_content_validation_failed:
    'No se descarg\u00f3 el archivo: su validaci\u00f3n fall\u00f3.',
  document_integrity_incident_not_found: 'El incidente de integridad no est\u00e1 disponible.',
  user_already_exists:
    'Ya existe una cuenta con ese correo. Usa otro correo para el nuevo integrante.',
  bootstrap_closed: 'El despacho ya tiene un administrador. Inicia sesi\u00f3n con tu cuenta.',
  invalid_input: 'Revisa los datos ingresados y los l\u00edmites de cada campo.',
  case_not_found: 'El expediente no est\u00e1 disponible o ya no tienes acceso.',
  user_not_found: 'No se encontr\u00f3 un usuario activo con ese identificador.',
  user_revision_conflict:
    'La cuenta cambi\u00f3. Consulta su revisi\u00f3n actual antes de confirmar.',
  last_active_owner:
    'Debe conservarse al menos un administrador activo; no puedes retirar al \u00faltimo.',
  user_access_version_exhausted: 'La cuenta alcanz\u00f3 el l\u00edmite de cambios de acceso.',
  invalid_document_metadata: 'Revisa el tipo, la clasificaci\u00f3n y las etiquetas ingresadas.',
  document_metadata_revision_exhausted:
    'Este documento ha alcanzado el l\u00edmite de cambios de clasificaci\u00f3n. No se pueden registrar m\u00e1s cambios.',
  invalid_document_name: 'El nombre del documento no es v\u00e1lido.',
};

export function createApi(fetcher = globalThis.fetch, onExpired = () => {}) {
  let token = '';
  let principalId = null;
  let sessionVersion = 0;
  let authAttemptVersion = 0;
  let observedLogin = false;
  let currentChallenge = null;
  let certificateCapture = null;
  let expectedOwnerId = null;
  let locallyClosed = false;
  let sessionGuard = () => true;
  function invalidateAuthentication() {
    currentChallenge = null;
    certificateCapture = null;
    expectedOwnerId = null;
    return ++authAttemptVersion;
  }
  function supersededAuthentication() {
    const error = new Error('Este intento de acceso fue sustituido. Inicia sesi\u00f3n de nuevo.');
    error.code = 'auth_attempt_superseded';
    return error;
  }
  function assertSession(version) {
    if (version !== sessionVersion)
      throw new Error('La sesi\u00f3n de esta solicitud termin\u00f3.');
  }
  function invalidateSession() {
    if (!locallyClosed || token) sessionVersion++;
    token = '';
    principalId = null;
    locallyClosed = true;
    invalidateAuthentication();
  }
  function assertAdmission(controlRoute) {
    if (locallyClosed || (!controlRoute && !sessionGuard())) {
      const error = new Error(messages.invalid_session);
      error.code = 'session_inactive';
      throw error;
    }
  }
  async function request(
    path,
    {
      method = 'GET',
      data,
      body,
      headers = {},
      protectedRoute = true,
      binary = false,
      controlRoute = false,
      revocationOnly = false,
    } = {},
  ) {
    if (protectedRoute) assertAdmission(controlRoute);
    const requestVersion = sessionVersion;
    const assertCurrentSession = () => {
      if (protectedRoute && !revocationOnly) assertSession(requestVersion);
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
      if (response.status === 401 && protectedRoute && requestVersion === sessionVersion) {
        invalidateSession();
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
          ...(binary === 'report'
            ? {
                reportId: response.headers.get('X-Report-Id'),
                digest: response.headers.get('X-Report-Digest'),
                snapshotDigest: response.headers.get('X-Report-Snapshot-Digest'),
                contentType: response.headers.get('Content-Type'),
                contentLength: response.headers.get('Content-Length'),
              }
            : {}),
          ...(binary === 'content'
            ? {
                caseId: response.headers.get('X-Case-Id'),
                contentType: response.headers.get('Content-Type'),
              }
            : {}),
        }
      : response.status === 204
        ? null
        : await response.json();
    assertCurrentSession();
    return result;
  }
  const post = (path, data, protectedRoute = true) =>
    request(path, { method: 'POST', data, protectedRoute });
  async function authenticate(path, data, attempt, accept = (value) => value) {
    const assertAttempt = () => {
      if (attempt !== authAttemptVersion) throw supersededAuthentication();
    };
    try {
      const result = await post(path, data, false);
      assertAttempt();
      return accept(result);
    } catch (failure) {
      assertAttempt();
      const certificate = expectedOwnerId !== null || path.startsWith('/auth/certificate-login/');
      invalidateAuthentication();
      if (certificate)
        failure.message =
          'El acceso con certificado no pudo confirmarse. Prepara un nuevo intento.';
      throw failure;
    }
  }
  return {
    invalidateSession,
    cancelAuthentication() {
      observedLogin = true;
      invalidateAuthentication();
    },
    async certificateLoginAvailable() {
      return ownerLoginAvailability(
        await request('/auth/certificate-login/availability', {
          protectedRoute: false,
        }),
      );
    },
    async startCertificateLogin(selection) {
      observedLogin = true;
      const attempt = invalidateAuthentication();
      const selected = ownerLoginSelection(selection);
      return authenticate(
        '/auth/certificate-login/start',
        {
          owner_id: selected.ownerId,
          binding_id: selected.bindingId,
        },
        attempt,
        (value) => {
          const checked = ownerLoginChallenge(value, selected);
          certificateCapture = checked.challenge_token;
          expectedOwnerId = selected.ownerId;
          return checked;
        },
      );
    },
    async proveCertificateLogin(challenge_token, signature_base64) {
      if (certificateCapture === null || challenge_token !== certificateCapture)
        throw supersededAuthentication();
      ownerBase64(signature_base64, 384);
      certificateCapture = null;
      return authenticate(
        '/auth/certificate-login/proof',
        {
          challenge_token,
          signature_base64,
        },
        ++authAttemptVersion,
        (value) => {
          const checked = ownerLoginMfa(value);
          currentChallenge = checked.challenge_token;
          return checked;
        },
      );
    },
    setSessionGuard(guard) {
      if (typeof guard !== 'function') throw new TypeError('A session guard is required.');
      const installed = () => guard();
      sessionGuard = installed;
      return () => {
        if (sessionGuard === installed) sessionGuard = () => true;
      };
    },
    login(email, password) {
      observedLogin = true;
      return authenticate(
        '/auth/login',
        { email, password },
        invalidateAuthentication(),
        (value) => {
          currentChallenge =
            typeof value?.challenge_token === 'string' ? value.challenge_token : null;
          return value;
        },
      );
    },
    bootstrap: (email, password) =>
      authenticate('/auth/bootstrap', { email, password }, invalidateAuthentication()),
    async mfa(challenge_token, code, mode) {
      if (!['totp', 'recovery'].includes(mode))
        throw new Error('M\u00e9todo de verificaci\u00f3n no v\u00e1lido.');
      if (observedLogin && (currentChallenge === null || challenge_token !== currentChallenge))
        throw supersededAuthentication();
      currentChallenge = null;
      return authenticate(
        `/auth/mfa/${mode}`,
        { challenge_token, code },
        ++authAttemptVersion,
        (session) => {
          if (
            expectedOwnerId !== null &&
            (session?.user?.id !== expectedOwnerId ||
              session.user.role !== 'owner' ||
              typeof session.access_token !== 'string' ||
              !session.access_token)
          )
            throw new Error('La sesion no corresponde a la cuenta Owner seleccionada.');
          token = session.access_token;
          principalId = session.user?.id ?? null;
          locallyClosed = false;
          sessionVersion++;
          currentChallenge = null;
          certificateCapture = expectedOwnerId = null;
          return session;
        },
      );
    },
    me: () => request('/auth/me'),
    sessionStatus: () => request('/auth/session', { controlRoute: true }),
    recordActivity: () => post('/auth/activity'),
    logout() {
      const result = request('/auth/logout', {
        method: 'POST',
        controlRoute: true,
        revocationOnly: true,
      });
      invalidateSession();
      return result;
    },
    createUser: (email, password, role) => post('/users', { email, password, role }),
    ...caseApi(request),
    dashboard: () => dashboardApi(request),
    auditEvents: () => auditEventsApi(request),
    reports: () => caseReportsApi(request),
    judicialCalendars: () => judicialCalendarsApi(request),
    ownerCertificates(ownerId) {
      const version = sessionVersion;
      return ownerCertificatesApi((path, options) => {
        assertSession(version);
        if (principalId !== null && principalId !== ownerId)
          throw new Error('La cuenta del certificado no corresponde a la sesion.');
        return request(path, options);
      }, ownerId);
    },
    alerts: (actorId) => alertsApi(request, actorId),
    integrityIncidents: () => integrityIncidentsApi(request),
    members: () =>
      membersApi(request, (record) => {
        if (record.id !== principalId) return;
        invalidateSession();
        onExpired();
      }),
    audit: () => request('/audit/verify'),
  };
}
