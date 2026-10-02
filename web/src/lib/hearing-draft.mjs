import { descriptorKey } from './draft-descriptor.mjs';
import { canHearings } from './hearings.mjs';
import { hearingDraftFields, validateHearingDraft } from './hearing-draft-values.mjs';
import { stageSupportKey } from './stage-support-draft.mjs';

export const resultDraftIntent = (hearingId, source) =>
  JSON.stringify([hearingId, source?.result_id ?? null, source?.revision ?? null]);
export function pendingHearingDrafts(session, caseId, result = false, hearingId = null) {
  return (
    session?.registry
      .pending()
      .filter(
        (row) =>
          row.principalId === session.principal()?.id &&
          row.contextId === caseId &&
          row.editorKind === (result ? 'hearing-result' : 'hearing') &&
          (!result || JSON.parse(row.instanceId)[0] === hearingId),
      ) ?? []
  );
}
export function createHearingDraft({
  session,
  caseId,
  result = false,
  intent = null,
  capture,
  read,
}) {
  const principalId = session.principal()?.id;
  let alive = true,
    root = null,
    registration = null;
  function admitted() {
    const principal = session.principal();
    return (
      alive &&
      principal?.id === principalId &&
      canHearings(principal.role, 'manage') &&
      session.canAdmit()
    );
  }
  async function fresh() {
    if (!admitted()) return null;
    const record = await session.authorizeCase(caseId);
    if (!admitted()) return null;
    const administration = record.administration;
    if (
      record.id !== caseId ||
      administration?.case_id !== caseId ||
      !['active', 'closed'].includes(administration.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    const current = await read();
    return admitted()
      ? { ...current, closed: administration.administrative_status === 'closed' }
      : null;
  }
  function register(action, resourceId, revision) {
    if (!admitted()) throw new Error('La sesion no admite el borrador.');
    const next = {
      principalId,
      contextId: caseId,
      editorKind: result ? 'hearing-result' : 'hearing',
      resourceId,
      action,
      instanceId: intent,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: revision,
    };
    if (
      registration &&
      (descriptorKey(next) !== descriptorKey(root) || root.baseRevision !== revision)
    ) {
      registration.dispose();
      registration = null;
    }
    root = next;
    registration ??= session.registry.register(root, { fields: hearingDraftFields, capture });
  }
  async function restore(saved, apply) {
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
        if (!admitted()) throw new Error('La sesion ya no admite el borrador.');
        validateHearingDraft(value, saved, result);
        register(saved.action, saved.resourceId, saved.baseRevision);
        apply(value, context);
      },
    });
    if (failure) throw failure;
    return outcome;
  }
  function supportContext(field, canApply, observe, deny) {
    if (!root) return null;
    const ownerDraftKey = descriptorKey(root);
    const context = {
      principalId,
      caseId,
      ownerDraftKey,
      fieldPath: [...field],
      rowId: null,
      canApply: () =>
        admitted() && root !== null && descriptorKey(root) === ownerDraftKey && canApply(),
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
      else close();
    },
    dispose() {
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}

export async function refreshHearingReferences(
  draft,
  typed,
  documents,
  caseId,
  admitted,
  result = false,
) {
  const refs = result ? draft.attendees : draft.participants,
    rows = [];
  for (const ref of refs) {
    const row = await typed.participantRevision(ref.participant_id, ref.revision);
    if (!admitted()) return null;
    rows.push({
      ...row,
      profile: row.profile ? 'typed' : 'manual',
      kind: row.profile?.kind,
      retained: true,
    });
  }
  const ref = result ? draft.provenance.support : draft.support;
  if (ref) {
    const id = ref.document_id ?? ref.id,
      scoped = documents.version(id, ref.version);
    try {
      const value = await scoped.detail();
      if (!admitted()) return null;
      if (
        value.case_id !== caseId ||
        value.id !== id ||
        value.version !== ref.version ||
        value.digest !== ref.digest
      )
        throw new Error('El soporte no coincide con la version conservada.');
    } catch (failure) {
      throw Object.assign(new Error(failure.message), {
        code: failure.code,
        status: failure.status,
        draftReference: 'support',
      });
    } finally {
      scoped.dispose();
    }
  }
  return admitted() ? rows : null;
}

async function readOwnedReference(read) {
  try {
    return await read();
  } catch (failure) {
    if ([403, 404].includes(failure.status)) failure.draftReference = 'owner';
    throw failure;
  }
}
export async function readHearingDraftContext(api, base) {
  const context = await api.context();
  const current = base ? await readOwnedReference(() => api.get(base.id)) : null;
  return { context, current };
}
export async function readResultDraftContext(hearings, scoped, hearingId, base) {
  const hearing = await readOwnedReference(() => hearings.get(hearingId));
  const current = base ? await readOwnedReference(() => scoped.get(base.id)) : null;
  return { hearing, current };
}

export async function readResultDraftSources(api, hearings, caseId, anchor, source, admitted) {
  const origin = await hearings.revision(anchor.hearing_id, anchor.revision);
  if (!admitted()) return null;
  if (
    origin.values_digest !== anchor.values_digest ||
    origin.receipt.submission_digest !== anchor.submission_digest
  )
    throw new Error('La programacion no coincide con la referencia conservada.');
  if (source) {
    const previousApi = api.caseHearingResults(caseId, source.hearing_id);
    try {
      const previous = await previousApi.revision(source.result_id, source.revision);
      if (!admitted()) return null;
      if (
        previous.values_digest !== source.values_digest ||
        previous.receipt.submission_digest !== source.submission_digest
      )
        throw new Error('El antecedente no coincide con la referencia conservada.');
    } finally {
      previousApi.dispose();
    }
  }
  return origin;
}
