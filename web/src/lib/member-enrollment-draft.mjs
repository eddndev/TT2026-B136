import { descriptorKey } from './draft-descriptor.mjs';
import { memberObject, memberRole } from './members-values.mjs';

const contextId = 'member-enrollment';

function value(input) {
  memberObject(input, ['email', 'role', 'uncertain']);
  if (typeof input.email !== 'string' || typeof input.uncertain !== 'boolean')
    throw new Error('El borrador del alta no es compatible.');
  memberRole(input.role);
  return { email: input.email, role: input.role, uncertain: input.uncertain };
}

export async function authorizeEnrollment(api, principalId, admitted) {
  if (!admitted()) return false;
  const principal = await api.me();
  if (!admitted()) return false;
  if (principal?.id !== principalId || principal.role !== 'owner')
    throw Object.assign(new Error('La cuenta ya no tiene permiso para crear integrantes.'), {
      status: 403,
    });
  return true;
}

export async function findEnrollmentAccount(api, members, principalId, email, admitted) {
  if (!(await authorizeEnrollment(api, principalId, admitted))) return null;
  const prefix = email.trim().toLowerCase();
  if (!prefix) throw new Error('El intento no conserva un correo consultable.');
  let cursor,
    previous = '',
    found = false;
  const cursors = new Set();
  do {
    const page = await members.list({
      limit: 20,
      status: 'all',
      email_prefix: prefix,
      ...(cursor ? { cursor } : {}),
    });
    if (!admitted()) return null;
    for (const row of page.items) {
      if (row.id <= previous) throw new Error('El directorio cambio de orden. Consulta de nuevo.');
      previous = row.id;
      if (row.email === prefix) found = true;
    }
    cursor = page.next_cursor;
    if (cursor) {
      if (cursors.has(cursor)) throw new Error('La continuacion del directorio no es valida.');
      cursors.add(cursor);
    }
  } while (cursor !== null);
  return admitted() ? { found } : null;
}

export function createMemberEnrollmentDraft({ session, principalId, capture }) {
  const descriptor = {
    principalId,
    contextId,
    editorKind: 'member-enrollment',
    resourceId: null,
    action: 'create',
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: null,
  };
  const key = descriptorKey(descriptor);
  let alive = true,
    registration = null;
  const admitted = () =>
    alive &&
    session.principal()?.id === principalId &&
    session.principal()?.role === 'owner' &&
    session.canAdmit();
  function register() {
    if (!admitted()) throw new Error('La sesion no admite este borrador de alta.');
    registration ??= session.registry.register(descriptor, {
      fields: ['email', 'role', 'uncertain'],
      capture: () => value(capture()),
    });
  }
  function close() {
    session.registry.closeEditor(key);
    registration?.dispose();
    registration = null;
  }
  return {
    admitted,
    register,
    close,
    retain() {
      return admitted() && registration ? registration.capture() : { status: 'stale' };
    },
    pending: () => session.registry.pending().some((entry) => entry.key === key),
    async restore(authorize, apply) {
      let failure;
      const result = await session.registry.restore(key, {
        authorize: async (entry) => {
          try {
            return (
              entry.schemaVersion === 1 &&
              entry.baseRevision === null &&
              (await authorize()) &&
              admitted()
            );
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(input) {
          if (!admitted()) throw new Error('La sesion ya no admite el alta.');
          const saved = value(input);
          apply(saved);
          register();
        },
      });
      if (failure) throw failure;
      return result;
    },
    deny() {
      session.registry.denyContext(contextId);
      close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
