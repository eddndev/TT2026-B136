import { descriptorKey } from './draft-descriptor.mjs';

function descriptor(context) {
  return {
    principalId: context.principalId,
    contextId: context.caseId,
    editorKind: 'participant-support',
    resourceId: null,
    action: 'edit',
    instanceId: null,
    ownerDraftKey: context.ownerDraftKey,
    fieldPath: [...context.fieldPath],
    rowId: context.rowId ?? null,
    schemaVersion: 1,
    baseRevision: null,
  };
}

export function participantSupportKey(context) {
  return descriptorKey(descriptor(context));
}

export function discardOwnedSupport(session, context) {
  if (session && context) session.registry.closeEditor(participantSupportKey(context));
}

function validate(value) {
  if (
    value?.picker !== null &&
    (typeof value?.picker?.name !== 'string' || typeof value?.picker?.query !== 'string')
  )
    throw new TypeError('Invalid support selector draft.');
}

export function createParticipantSupportDraft({ session, context, capture }) {
  const initial = descriptor(context());
  const key = descriptorKey(initial);
  let alive = true,
    registration = null;

  function admitted() {
    const current = context();
    return (
      alive &&
      current !== null &&
      current !== undefined &&
      participantSupportKey(current) === key &&
      session.principal()?.id === initial.principalId &&
      session.canAdmit() &&
      current.canApply() === true
    );
  }

  async function authorize() {
    if (!admitted()) return false;
    const allowed = await context().authorize();
    return allowed === true && admitted();
  }

  function register() {
    if (!admitted()) throw new Error('No se pudo confirmar el propietario del soporte.');
    if (!registration)
      registration = session.registry.register(initial, {
        fields: ['picker'],
        capture,
      });
  }

  async function restore(apply) {
    let failure;
    const result = await session.registry.restore(key, {
      authorize: async (entry) => {
        try {
          return entry.schemaVersion === 1 && (await authorize());
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(value) {
        if (!admitted()) throw new Error('El soporte ya no pertenece al editor activo.');
        validate(value);
        register();
        apply(value);
      },
    });
    if (failure) throw failure;
    return result;
  }

  return {
    key,
    admitted,
    authorize,
    register,
    restore,
    pending: () => session.registry.pending().some((entry) => entry.key === key),
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
