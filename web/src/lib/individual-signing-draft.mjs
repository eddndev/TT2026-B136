import { descriptorKey } from './draft-descriptor.mjs';

export function createSigningDraft(session, context, kind, capture) {
  let alive = true,
    registration = null;
  const descriptor = {
    principalId: context.accountId,
    contextId: kind === 'document' ? context.caseId : 'personal-document-signing-certificate',
    editorKind: 'individual-signing-' + kind,
    resourceId: kind === 'document' ? context.documentId : context.accountId,
    action: 'sign',
    instanceId: kind === 'document' ? String(context.version) : null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: kind === 'document' ? context.version : 0,
  };
  const key = descriptorKey(descriptor);
  const admitted = () =>
    alive &&
    session?.principal()?.id === context.accountId &&
    session.principal()?.role === 'litigator' &&
    session.canAdmit();
  function register() {
    if (!admitted()) throw new Error('La sesion no admite este intento.');
    registration ??= session.registry.register(descriptor, {
      fields: ['state', 'digest'],
      capture,
    });
  }
  const pending = () => session?.registry.pending().find((value) => value.key === key) ?? null;
  return {
    admitted,
    pending,
    register,
    async restore(authorize, apply) {
      const saved = pending();
      if (!saved || !admitted()) return { status: 'missing' };
      const result = await session.registry.restore(saved.key, {
        authorize: async (found) =>
          found.schemaVersion === 1 &&
          found.baseRevision === descriptor.baseRevision &&
          descriptorKey(found) === key &&
          (await authorize()) === true &&
          admitted(),
        apply(value) {
          if (
            !admitted() ||
            !value ||
            Object.keys(value).length !== 2 ||
            value.digest !== (context.digest ?? null) ||
            !value.state ||
            typeof value.state !== 'object'
          )
            throw new Error('El intento no corresponde a este contexto.');
          apply(value);
        },
      });
      if (result.status === 'restored') register();
      return result;
    },
    close() {
      session?.registry.closeEditor(key);
      registration?.dispose();
      registration = null;
    },
    dispose(preserve = false) {
      if (preserve && admitted()) registration?.capture();
      alive = false;
      registration?.dispose();
      registration = null;
    },
  };
}
