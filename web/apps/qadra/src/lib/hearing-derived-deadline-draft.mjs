import { descriptorKey } from './draft-descriptor.mjs';
import { stageSupportKey } from './stage-support-draft.mjs';
import { canHearings } from './hearings.mjs';
import {
  factInvalid as invalid,
  factRevision as revision,
  factSame as same,
} from './procedural-fact-primitives.mjs';
import { uuid } from './hearing-derived-deadline-command.mjs';
import {
  captureHearingDerivedDeadlineDraft,
  hearingDerivedDeadlineDraftFields,
  validateHearingDerivedDeadlineDraft,
} from './hearing-derived-deadline-draft-values.mjs';

export { captureHearingDerivedDeadlineDraft };

function owns(descriptor, principalId, caseId, hearingId) {
  return (
    descriptor !== null &&
    typeof descriptor === 'object' &&
    descriptor.principalId === principalId &&
    descriptor.contextId === caseId &&
    descriptor.resourceId === hearingId &&
    descriptor.editorKind === 'hearing-derived-deadline' &&
    descriptor.action === 'register' &&
    descriptor.schemaVersion === 1 &&
    descriptor.ownerDraftKey === null &&
    descriptor.rowId === null &&
    Array.isArray(descriptor.fieldPath) &&
    descriptor.fieldPath.length === 0
  );
}

export function pendingHearingDerivedDeadlineDrafts(session, caseId, hearingId) {
  const principal = session?.principal();
  if (!principal || !canHearings(principal.role, 'manage') || !session.canAdmit()) return [];
  return session.registry.pending().filter((row) => owns(row, principal.id, caseId, hearingId));
}

export function createHearingDerivedDeadlineDraft({ session, caseId, hearingId, capture, read }) {
  uuid(caseId);
  uuid(hearingId);
  const principalId = session.principal()?.id;
  let alive = true,
    root = null,
    registration = null;

  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      !!principal &&
      principal.id === principalId &&
      canHearings(principal.role, 'manage') &&
      session.canAdmit()
    );
  }

  async function authorizeCase() {
    if (!admitted()) return null;
    const current = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const administration = current?.administration;
    if (
      current?.id !== caseId ||
      administration?.case_id !== caseId ||
      !['active', 'closed'].includes(administration.administrative_status)
    )
      invalid('No se pudo confirmar el expediente actual.');
    return { closed: administration.administrative_status === 'closed' };
  }

  async function fresh() {
    const context = await authorizeCase();
    if (context === null) return null;
    if (!read) return context;
    const current = await read();
    if (!admitted()) return null;
    if (!current || typeof current !== 'object' || Array.isArray(current)) invalid();
    return { ...current, ...context };
  }

  function register(resultId, anchorRevision) {
    if (!admitted()) invalid('La sesion no admite el borrador conjunto.');
    uuid(resultId);
    revision(anchorRevision);
    const next = {
      principalId,
      contextId: caseId,
      resourceId: hearingId,
      editorKind: 'hearing-derived-deadline',
      action: 'register',
      instanceId: resultId,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: anchorRevision,
    };
    if (
      registration &&
      (descriptorKey(next) !== descriptorKey(root) || root.baseRevision !== anchorRevision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
    registration ??= session.registry.register(root, {
      fields: hearingDerivedDeadlineDraftFields,
      capture() {
        const value = captureHearingDerivedDeadlineDraft(capture());
        validateHearingDerivedDeadlineDraft(value, root, principalId);
        return value;
      },
    });
  }

  async function restore(saved, apply) {
    if (!admitted() || !owns(saved, principalId, caseId, hearingId)) return { status: 'denied' };
    let context, failure, descriptor;
    const outcome = await session.registry.restore(saved.key, {
      async authorize(stored) {
        descriptor = stored;
        if (
          !admitted() ||
          !owns(stored, principalId, caseId, hearingId) ||
          saved.key !== descriptorKey(stored) ||
          !same({ ...saved, key: undefined }, { ...stored, key: undefined })
        )
          return false;
        try {
          context = await authorizeCase();
          return context !== null && admitted();
        } catch (error) {
          failure = error;
          throw error;
        }
      },
      apply(stored) {
        if (!admitted()) invalid('La sesion ya no admite el borrador.');
        const value = captureHearingDerivedDeadlineDraft(stored);
        validateHearingDerivedDeadlineDraft(value, descriptor, principalId);
        register(value.resultId, descriptor.baseRevision);
        apply(value, context);
      },
    });
    if (failure) throw failure;
    return outcome;
  }

  function supportContext(field, canApply, observe, deny) {
    if (!root) return null;
    const ownerDraftKey = descriptorKey(root),
      baseRevision = root.baseRevision;
    const context = {
      principalId,
      caseId,
      ownerDraftKey,
      fieldPath: [...field],
      rowId: null,
      canApply: () =>
        admitted() &&
        root !== null &&
        descriptorKey(root) === ownerDraftKey &&
        root.baseRevision === baseRevision &&
        canApply(),
      async authorize() {
        if (!context.canApply()) return false;
        try {
          const current = await fresh();
          if (!current || !context.canApply()) return false;
          observe(current);
          return context.canApply();
        } catch (error) {
          if (context.canApply() && [403, 404].includes(error.status)) deny(error);
          throw error;
        }
      },
    };
    return context;
  }

  function close() {
    if (root) session.registry.closeEditor(descriptorKey(root));
    registration?.dispose();
    root = registration = null;
  }

  return {
    admitted,
    fresh,
    register,
    restore,
    supportContext,
    close,
    discardSupport(field) {
      if (root)
        session.registry.closeEditor(
          stageSupportKey({
            principalId,
            caseId,
            ownerDraftKey: descriptorKey(root),
            fieldPath: field,
          }),
        );
    },
    deny(failure) {
      if (failure.code === 'case_not_found' || failure.status === 403)
        session.registry.denyContext(caseId);
      close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
