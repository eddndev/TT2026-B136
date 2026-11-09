import { descriptorKey } from './draft-descriptor.mjs';
import { authorizeEnrollment, pendingAtSchema } from './member-enrollment-draft.mjs';
// Schema 2: issue intents carry the invitee name written by the Owner.
const schemaVersion = 2;
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
    schemaVersion,
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
    if (!admitted()) throw new Error('La sesi\u00f3n no admite invitaciones.');
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
    pending: () => pendingAtSchema(session.registry, key, schemaVersion),
    async restore(apply) {
      return session.registry.restore(key, {
        authorize: async (entry) =>
          entry.schemaVersion === schemaVersion &&
          (await authorizeEnrollment(api, principalId, admitted)),
        apply(value) {
          if (!admitted()) throw new Error('La sesi\u00f3n termin\u00f3.');
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
