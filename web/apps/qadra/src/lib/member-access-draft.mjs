import { descriptorKey } from './draft-descriptor.mjs';
import { memberObject, memberSummary, memberRole, memberUuid } from './members-values.mjs';

const contextId = 'member-access';

function descriptor(principalId, id) {
  return {
    principalId,
    contextId,
    editorKind: 'member-access',
    resourceId: id,
    action: 'replace',
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: null,
  };
}

export function discardMemberAccess(session, principalId, failure, id) {
  if (!session) return;
  if (failure.status === 403) session.registry.denyContext(contextId);
  else if (failure.status === 404 && id)
    session.registry.closeEditor(descriptorKey(descriptor(principalId, id)));
}

export async function freshMemberAccess({ api, members, principalId, id, admitted }) {
  if (!admitted()) return null;
  const principal = await api.me();
  if (!admitted()) return null;
  memberObject(principal, ['id', 'email', 'role']);
  memberUuid(principal.id);
  if (principal.id !== principalId || principal.role !== 'owner') {
    const failure = new Error('Ya no tienes permiso para administrar el acceso de cuentas.');
    failure.status = 403;
    failure.code = 'permission_denied';
    throw failure;
  }
  const current = await members.get(id);
  return admitted() ? memberSummary(current, id) : null;
}

function validateDraft(value, id, current) {
  memberObject(value, ['base', 'role', 'status', 'unconfirmed']);
  memberSummary(value.base, id);
  memberRole(value.role);
  if (!['active', 'inactive'].includes(value.status) || typeof value.unconfirmed !== 'boolean')
    throw new TypeError('Invalid account access draft.');
  if (BigInt(current.revision) < BigInt(value.base.revision))
    throw new Error('La cuenta actual no confirma la revisi\u00f3n del borrador.');
}

export function createMemberAccessDraft({ session, principalId, id, capture }) {
  const value = descriptor(principalId, id),
    key = descriptorKey(value);
  let alive = true,
    registration = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive && principal?.id === principalId && principal.role === 'owner' && session.canAdmit()
    );
  }
  function register() {
    if (!admitted()) throw new Error('La sesi\u00f3n no admite el borrador de acceso.');
    if (!registration)
      registration = session.registry.register(value, {
        fields: ['base', 'role', 'status', 'unconfirmed'],
        capture,
      });
  }
  return {
    admitted,
    register,
    pending: () => session.registry.pending().some((entry) => entry.key === key),
    async restore(current, apply) {
      let failure;
      const result = await session.registry.restore(key, {
        authorize: (entry) =>
          admitted() &&
          entry.schemaVersion === 1 &&
          entry.baseRevision === null &&
          current.id === id,
        apply(saved) {
          try {
            if (!admitted()) throw new Error('La sesi\u00f3n no admite el borrador de acceso.');
            validateDraft(saved, id, current);
            register();
            apply(saved);
          } catch (error) {
            failure = error;
            throw error;
          }
        },
      });
      if (failure) throw failure;
      return result;
    },
    close() {
      session.registry.closeEditor(key);
      registration?.dispose();
      registration = null;
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
