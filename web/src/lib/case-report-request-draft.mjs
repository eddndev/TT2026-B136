import { descriptorKey } from './draft-descriptor.mjs';
import { factObject } from './procedural-fact-primitives.mjs';
import { reportCommand, reportUuid } from './case-report-values.mjs';
import { canReports, reportScope } from './case-reports-presentation.mjs';

export function reportDraftValue(value, role) {
  factObject(value, ['draft', 'pending', 'role']);
  const fields = ['from', 'before', 'status', 'assigned'];
  factObject(value.draft, [...fields, 'reportType'], fields);
  if (
    Object.hasOwn(value.draft, 'reportType') &&
    !['case_state', 'litigator_activity'].includes(value.draft.reportType)
  )
    throw new Error('La solicitud pendiente no corresponde a un tipo de informe disponible.');
  if (value.role !== role) {
    const error = new Error('La solicitud pendiente corresponde a otro rol de la cuenta.');
    error.status = 403;
    throw error;
  }
  if (Object.values(value.draft).some((field) => typeof field !== 'string'))
    throw new Error('La solicitud pendiente no corresponde al acceso actual.');
  if (value.draft.assigned) reportUuid(value.draft.assigned);
  if (value.pending !== null) reportCommand(value.pending);
  return structuredClone(value);
}

export async function freshReportRequest(api, scoped, user, admitted, types = ['case_state']) {
  if (!admitted()) return null;
  const principal = await api.me();
  if (!admitted()) return null;
  factObject(principal, ['id', 'email', 'role']);
  reportUuid(principal.id);
  if (principal.id !== user.id || principal.role !== user.role || !canReports(principal.role)) {
    const error = new Error('La cuenta o sus permisos ya no admiten esta solicitud de informe.');
    error.status = 403;
    throw error;
  }
  const choices = {};
  for (const type of types) {
    const lawyers = [];
    let afterId;
    do {
      const page = await scoped.litigators({
        limit: 20,
        ...(afterId ? { after_id: afterId } : {}),
        ...(type === 'litigator_activity' ? { report_type: type } : {}),
      });
      if (!admitted()) return null;
      if (page.scope !== reportScope(principal.role)) {
        const error = new Error('El alcance de litigantes no corresponde al acceso actual.');
        error.status = 403;
        throw error;
      }
      lawyers.push(...page.litigators);
      afterId = page.has_more ? page.next_after_id : null;
    } while (afterId !== null);
    choices[type] = lawyers;
  }
  return admitted() ? choices : null;
}

export function createReportRequestDraft({ session, user, capture }) {
  const descriptor = {
      principalId: user.id,
      contextId: 'case-report-request',
      editorKind: 'case-report-request',
      resourceId: null,
      action: 'request',
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: null,
    },
    key = descriptorKey(descriptor);
  let alive = true,
    registration = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === user.id &&
      principal.role === user.role &&
      canReports(principal.role) &&
      session.canAdmit()
    );
  }
  function register() {
    if (!admitted()) throw new Error('La sesion no admite la solicitud pendiente.');
    registration ??= session.registry.register(descriptor, {
      fields: ['draft', 'pending', 'role'],
      capture: () => reportDraftValue(capture(), user.role),
    });
  }
  return {
    admitted,
    register,
    pending: () => session.registry.pending().some((row) => row.key === key),
    async restore(fresh, apply) {
      let context, failure;
      const result = await session.registry.restore(key, {
        authorize: async (entry) => {
          try {
            context = await fresh();
            return (
              admitted() &&
              entry.schemaVersion === 1 &&
              entry.baseRevision === null &&
              context !== null
            );
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(value) {
          try {
            if (!admitted()) throw new Error('La sesion ya no admite la solicitud pendiente.');
            const saved = reportDraftValue(value, user.role);
            register();
            apply(saved, context);
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
