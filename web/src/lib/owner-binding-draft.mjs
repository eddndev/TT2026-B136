import { descriptorKey } from './draft-descriptor.mjs';
import { ownerPreparation, ownerReceipt, ownerUuid } from './owner-certificate-values.mjs';
import { registrationIntent, withdrawalIntent } from './owner-certificate-intent.mjs';

const contextId = 'personal-owner-certificate';
const fields = ['prepared', 'signatureBase64', 'signatureName', 'command', 'mode', 'original'];
export const ownerDrafts = (session, principalId) =>
  session?.registry
    .pending()
    .filter(
      (row) =>
        row.principalId === principalId &&
        row.contextId === contextId &&
        row.editorKind === 'owner-certificate' &&
        row.instanceId === null &&
        row.ownerDraftKey === null &&
        ['register', 'withdraw'].includes(row.action),
    ) ?? [];
export const denyOwnerDrafts = (session) => session?.registry.denyContext(contextId);

export function ownerFailure(error) {
  if (error?.status === 401)
    return 'La sesion termino. Vuelve a ingresar para comprobar el resultado.';
  if (error?.status === 403) return 'Tu cuenta ya no tiene permiso para esta consulta.';
  if (error?.status === 404) return 'No se encontro un recibo para este vinculo.';
  if (error?.status === 409) return 'El estado cambio. Conserva el intento y comprueba su recibo.';
  if ([400, 413, 422].includes(error?.status))
    return 'El servidor no admitio el material. Revisa el certificado publico y la firma separada.';
  return 'No se pudo confirmar la operacion. Conserva el intento y consulta su recibo exacto.';
}

export async function ownerBlobBase64(blob) {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  return btoa(String.fromCharCode(...bytes));
}

export async function freshOwnerBinding(api, scoped, principalId, admitted) {
  if (!admitted()) return null;
  const principal = await api.me();
  if (!admitted()) return null;
  if (principal?.id !== principalId || principal.role !== 'owner')
    throw Object.assign(new Error('La cuenta ya no admite esta consulta.'), { status: 403 });
  const current = await scoped.current();
  return admitted() ? { current } : null;
}

function validate(value, descriptor) {
  if (
    !value ||
    Object.keys(value).length !== fields.length ||
    fields.some((key) => !Object.hasOwn(value, key)) ||
    !['editing', 'uncertain', 'rejected', 'conflict'].includes(value.mode) ||
    typeof value.signatureBase64 !== 'string' ||
    typeof value.signatureName !== 'string' ||
    value.signatureName.length > 124
  )
    throw new Error('El borrador no es compatible.');
  ownerUuid(descriptor.resourceId);
  let expected = null;
  if (descriptor.action === 'register') {
    if (descriptor.baseRevision !== 0 || value.original !== null)
      throw new Error('Borrador invalido.');
    if (value.prepared) {
      ownerPreparation(value.prepared, descriptor.principalId, descriptor.resourceId);
      if (value.signatureBase64)
        expected = registrationIntent(value.prepared, value.signatureBase64);
    } else if (value.signatureBase64 || value.command) throw new Error('Falta la preparacion.');
  } else {
    if (
      descriptor.baseRevision !== 1 ||
      value.prepared !== null ||
      value.signatureBase64 ||
      value.signatureName
    )
      throw new Error('Borrador invalido.');
    expected = withdrawalIntent(
      ownerReceipt(value.original, descriptor.principalId, descriptor.resourceId, true),
    );
  }
  if (
    (value.mode === 'uncertain' && !value.command) ||
    (value.command && JSON.stringify(value.command) !== JSON.stringify(expected))
  )
    throw new Error('El intento no conserva la evidencia original.');
}

export function createOwnerBindingDraft({ session, principalId, bindingId, action, capture }) {
  let alive = true,
    registration = null;
  const descriptor = {
    principalId,
    contextId,
    editorKind: 'owner-certificate',
    resourceId: bindingId,
    action,
    instanceId: null,
    ownerDraftKey: null,
    fieldPath: [],
    rowId: null,
    schemaVersion: 1,
    baseRevision: action === 'register' ? 0 : 1,
  };
  const admitted = () =>
    alive &&
    session.principal()?.id === principalId &&
    session.principal()?.role === 'owner' &&
    session.canAdmit();
  function register() {
    if (!admitted()) throw new Error('La sesion no admite este borrador.');
    registration ??= session.registry.register(descriptor, { fields, capture });
  }
  return {
    admitted,
    register,
    async restore(saved, fresh, apply) {
      let context, failure;
      const result = await session.registry.restore(saved.key, {
        authorize: async (found) => {
          if (found.schemaVersion !== 1 || descriptorKey(found) !== descriptorKey(descriptor))
            return false;
          try {
            context = await fresh();
            return context !== null && admitted();
          } catch (error) {
            failure = error;
            throw error;
          }
        },
        apply(value) {
          if (!admitted()) throw new Error('La sesion termino.');
          validate(value, saved);
          apply(value, context);
          register();
        },
      });
      if (failure) throw failure;
      return result;
    },
    close() {
      session.registry.closeEditor(descriptorKey(descriptor));
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
