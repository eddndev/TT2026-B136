import { descriptorKey } from './draft-descriptor.mjs';
import { authorizeEnrollment } from './member-enrollment-draft.mjs';
export function invitationOwnerDraft(session, api, principalId, capture) {
  const descriptor = {
    principalId,
    contextId: 'member-invitations',
    editorKind: 'member-invitations',
    resourceId: null,
    action: 'issue-or-resend',
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: null,
  };
  const key = descriptorKey(descriptor);
  let registration,
    alive = true;
  const admitted = () =>
    alive &&
    session.canAdmit() &&
    session.principal()?.id === principalId &&
    session.principal()?.role === 'owner';
  function register() {
    if (!admitted()) throw new Error('La sesion no admite invitaciones.');
    registration ??= session.registry.register(descriptor, { fields: ['intent'], capture });
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
    pending: () => session.registry.pending().some((value) => value.key === key),
    async restore(apply) {
      return session.registry.restore(key, {
        authorize: () => authorizeEnrollment(api, principalId, admitted),
        apply(value) {
          if (!admitted()) throw new Error('La sesion termino.');
          apply(value.intent);
          register();
        },
      });
    },
    dispose() {
      if (admitted()) registration?.capture();
      registration?.dispose();
      alive = false;
    },
  };
}
