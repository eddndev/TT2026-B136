import { descriptorKey } from './draft-descriptor.mjs';
import { calendarDraftFields, validateCalendarDraft } from './calendar-draft-values.mjs';

const contextId = 'judicial-calendars';
export function pendingCalendarDrafts(session) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.contextId === contextId &&
          row.editorKind === 'judicial-calendar' &&
          row.principalId === session.principal()?.id,
      ) ?? []
  );
}
export function denyCalendarDrafts(session) {
  session?.registry.denyContext(contextId);
}
export async function freshCalendarOwner({ api, scoped, principalId, resourceId, admitted }) {
  if (!admitted()) return null;
  const principal = await api.me();
  if (!admitted()) return null;
  if (principal?.id !== principalId || principal.role !== 'owner')
    throw Object.assign(
      new Error('Ya no tienes permiso para administrar calendarios como Owner.'),
      {
        status: 403,
        code: 'permission_denied',
      },
    );
  const current = resourceId ? await scoped.get(resourceId) : await scoped.list({ limit: 1 });
  return admitted() ? { current: resourceId ? current : null } : null;
}
export function createCalendarDraft({ session, principalId, capture }) {
  let alive = true,
    registration = null,
    root = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive && principal?.id === principalId && principal.role === 'owner' && session.canAdmit()
    );
  }
  function register(action, resourceId, revision) {
    if (!admitted()) throw new Error('La sesion no admite el borrador de calendario.');
    const descriptor = {
      principalId,
      contextId,
      editorKind: 'judicial-calendar',
      resourceId,
      action,
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    if (
      registration &&
      (descriptorKey(descriptor) !== descriptorKey(root) || root.baseRevision !== revision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = descriptor;
    registration ??= session.registry.register(root, { fields: calendarDraftFields, capture });
  }
  return {
    admitted,
    register,
    async restore(saved, fresh, apply) {
      let context, failure;
      const outcome = await session.registry.restore(saved.key, {
        authorize: async (descriptor) => {
          try {
            context = await fresh();
            return descriptor.schemaVersion === 1 && context !== null && admitted();
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(value) {
          if (!admitted()) throw new Error('La sesion ya no admite el borrador de calendario.');
          validateCalendarDraft(value, saved);
          register(saved.action, saved.resourceId, saved.baseRevision);
          apply(value, context);
        },
      });
      if (failure) throw failure;
      return outcome;
    },
    close() {
      if (root) session.registry.closeEditor(descriptorKey(root));
      registration?.dispose();
      root = registration = null;
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
