import { descriptorKey } from './draft-descriptor.mjs';

function descriptor(context) {
  return {
    principalId: context.principalId,
    contextId: context.caseId,
    editorKind: 'stage-support',
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
export const stageSupportKey = (context) => descriptorKey(descriptor(context));

export function createStageSupportDraft({ session, context, capture }) {
  const initial = descriptor(context()),
    key = descriptorKey(initial);
  let alive = true,
    registration = null;
  function admitted() {
    const value = context();
    return (
      alive &&
      value &&
      stageSupportKey(value) === key &&
      session.principal()?.id === initial.principalId &&
      session.canAdmit() &&
      value.canApply() === true
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
      registration = session.registry.register(initial, { fields: ['picker'], capture });
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
        if (
          !admitted() ||
          (value?.picker !== null &&
            (typeof value?.picker?.name !== 'string' || typeof value?.picker?.query !== 'string'))
        )
          throw new Error('No se pudo recuperar el selector de soporte.');
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
    pending: () => session.registry.pending().some((row) => row.key === key),
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
