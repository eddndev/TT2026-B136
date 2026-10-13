import { descriptorKey } from './draft-descriptor.mjs';
import {
  alertPreferenceCommand,
  alertPreferenceValues,
  alertPreferencesEnvelope,
} from './alerts-preference-values.mjs';
import { canAlerts } from './alerts-presentation.mjs';

const contextId = 'personal-alert-preferences';
const fields = ['base', 'values', 'hours', 'mode', 'command'];

export function denyAlertPreferenceDrafts(session) {
  session?.registry.denyContext(contextId);
}

export async function freshAlertPreferences({ api, authorize, principalId, admitted }) {
  if (!admitted()) return null;
  const principal = await authorize();
  if (!admitted()) return null;
  if (principal?.id !== principalId || !canAlerts(principal.role))
    throw Object.assign(new Error('Tu cuenta ya no tiene acceso a estas preferencias.'), {
      status: 403,
      code: 'permission_denied',
    });
  const result = await api.preferences();
  if (!admitted()) return null;
  return alertPreferencesEnvelope(result, principalId).preferences;
}

function validate(value, descriptor) {
  if (
    !value ||
    Object.keys(value).length !== fields.length ||
    fields.some((key) => !Object.hasOwn(value, key))
  )
    throw new Error('El borrador de preferencias no es compatible.');
  alertPreferencesEnvelope({ preferences: value.base }, descriptor.principalId);
  alertPreferenceValues(value.values);
  if (
    value.base.revision !== descriptor.baseRevision ||
    !['editing', 'conflict', 'uncertain'].includes(value.mode) ||
    !value.hours ||
    Object.keys(value.hours).length !== 2 ||
    ['hearing_upcoming', 'deadline_upcoming'].some((key) => typeof value.hours[key] !== 'string') ||
    (value.mode === 'uncertain' && !value.command)
  )
    throw new Error('El borrador no conserva su revision o sus horas originales.');
  if (value.command) {
    alertPreferenceCommand(value.command);
    if (value.command.expected_revision < value.base.revision)
      throw new Error('El comando no corresponde a la revision del borrador.');
  }
}

export function createAlertPreferenceDraft({ session, principalId, capture }) {
  let alive = true,
    registration = null,
    root = null;
  const pending = () =>
    session.registry
      .pending()
      .find(
        (row) =>
          row.principalId === principalId &&
          row.contextId === contextId &&
          row.editorKind === 'alert-preferences' &&
          row.resourceId === principalId &&
          row.action === 'replace' &&
          row.instanceId === null &&
          row.ownerDraftKey === null,
      );
  function admitted() {
    const principal = session.principal();
    return (
      alive && principal?.id === principalId && canAlerts(principal.role) && session.canAdmit()
    );
  }
  function register(revision) {
    if (!admitted()) throw new Error('La sesion no admite el borrador de preferencias.');
    if (registration && root.baseRevision !== revision) {
      registration.dispose();
      registration = null;
    }
    root = {
      principalId,
      contextId,
      editorKind: 'alert-preferences',
      resourceId: principalId,
      action: 'replace',
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    registration ??= session.registry.register(root, { fields, capture });
  }
  return {
    admitted,
    pending,
    register,
    async restore(saved, fresh, apply) {
      let context, failure;
      const result = await session.registry.restore(saved.key, {
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
          if (!admitted()) throw new Error('La sesion ya no admite estas preferencias.');
          validate(value, saved);
          register(saved.baseRevision);
          apply(value, context);
        },
      });
      if (failure) throw failure;
      return result;
    },
    deny() {
      denyAlertPreferenceDrafts(session);
      registration?.dispose();
      root = registration = null;
    },
    close() {
      const saved = pending();
      if (saved) session.registry.closeEditor(saved.key);
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
