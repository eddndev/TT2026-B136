<script>
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { getContext, onDestroy } from 'svelte';
  import MetadataFields from './MetadataFields.svelte';
  import MetadataSummary from './MetadataSummary.svelte';
  import Icon from './Icon.svelte';
  import { metadataDraft, mutationError } from '../lib/document-metadata.mjs';
  import { can } from '../lib/documents.mjs';
  import { descriptorKey } from '../lib/draft-descriptor.mjs';
  import { discardDocumentDrafts } from '../lib/draft-document-access.mjs';
  const sessionDrafts = getContext('session-drafts');
  export let api;
  export let current;
  export let user;
  export let document;
  export let onconfirmed;
  export let ondenied;
  export let disabled = false;
  let dialog;
  let fields;
  let draft = metadataDraft();
  let expected = 0;
  let candidate = null;
  export let busy = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let blockedByCase = false;
  let registration = null,
    editorKey = null,
    restored = false,
    restoredClosed = false,
    restoreBlocked = false,
    needsReview = false,
    reviewAccepted = false,
    unconfirmed = false;
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
  }
  let alive = true;
  function admitted() {
    const principal = sessionDrafts ? sessionDrafts.principal() : user;
    return (
      alive &&
      principal?.id === user.id &&
      can(principal?.role, 'classify') &&
      (!sessionDrafts || sessionDrafts.canAdmit())
    );
  }
  function descriptor() {
    return {
      principalId: user.id,
      contextId: document.case_id,
      editorKind: 'document-metadata',
      resourceId: document.id,
      action: 'edit',
      instanceId: null,
      ownerDraftKey: null,
      fieldPath: [],
      rowId: null,
      schemaVersion: 1,
      baseRevision: expected,
    };
  }
  function suspended() {
    const key = descriptorKey(descriptor());
    return sessionDrafts?.registry.pending().find((entry) => entry.key === key);
  }
  export function hasSuspendedDraft() {
    return !!suspended();
  }
  export function discardDraft(failure) {
    if (!sessionDrafts) return null;
    return discardDocumentDrafts(
      sessionDrafts.registry,
      { contextId: document.case_id, editorKey: editorKey ?? descriptorKey(descriptor()) },
      failure,
    );
  }
  function deny(failure) {
    discardDraft(failure);
    ondenied(failure);
  }
  async function freshContext() {
    const result = await sessionDrafts.authorizeCase(document.case_id);
    if (!admitted()) return false;
    if (
      result.id !== document.case_id ||
      !['active', 'closed'].includes(result.administration?.administrative_status)
    )
      throw new Error('No se pudo confirmar el expediente actual.');
    restoredClosed = result.administration.administrative_status === 'closed';
    return true;
  }
  function captureDraft() {
    return { draft, expected, rawFields: fields.captureDraft(), unconfirmed };
  }
  function register() {
    if (sessionDrafts)
      registration = sessionDrafts.registry.register(descriptor(), {
        fields: ['draft', 'expected', 'rawFields', 'unconfirmed'],
        capture: captureDraft,
      });
  }
  function restoreDraft(value, latest, baseRevision) {
    if (
      !Number.isSafeInteger(value.expected) ||
      value.expected < 0 ||
      value.expected !== baseRevision ||
      typeof value.draft?.document_type !== 'string' ||
      typeof value.draft?.classification !== 'string' ||
      !Array.isArray(value.draft?.tags) ||
      value.draft.tags.some((tag) => typeof tag !== 'string') ||
      typeof value.unconfirmed !== 'boolean'
    )
      throw new TypeError('Invalid metadata draft.');
    fields.restoreDraft(value.rawFields);
    draft = value.draft;
    expected = value.expected;
    unconfirmed = value.unconfirmed;
    restored = true;
    needsReview = latest.metadata_revision !== expected || unconfirmed;
    reviewAccepted = false;
    candidate = needsReview || restoredClosed ? latest : null;
    conflict = needsReview;
    error = needsReview
      ? 'Compara los valores actuales y confirma tu decisi\u00f3n antes de volver a guardar.'
      : '';
    register();
    restoreBlocked = false;
  }
  export async function open() {
    if (busy || disabled || !admitted() || (dialog.open && !restoreBlocked)) return;
    const saved = suspended();
    if ($administration.closed && !saved) return;
    draft = metadataDraft(current);
    expected = current.metadata_revision;
    candidate = null;
    conflict = false;
    exhausted = false;
    error = '';
    restored = restoredClosed = needsReview = reviewAccepted = unconfirmed = false;
    restoreBlocked = !!saved;
    fields?.reset();
    editorKey = descriptorKey(descriptor());
    if (!dialog.open) dialog.showModal();
    busy = true;
    try {
      if (saved) {
        let latest;
        const result = await sessionDrafts.registry.restore(saved.key, {
          authorize: async (entry) => {
            try {
              if (entry.schemaVersion !== 1 || !admitted() || !(await freshContext())) return false;
              latest = await api.get();
              if (!admitted()) return false;
              if (latest.case_id !== document.case_id || latest.id !== document.id)
                throw new Error('No se pudo confirmar el documento actual.');
              await onconfirmed(latest);
              return admitted();
            } catch (failure) {
              if (alive) {
                error = failure.message;
                if ([403, 404].includes(failure.status)) deny(failure);
              }
              throw failure;
            }
          },
          apply: (value) => restoreDraft(value, latest, saved.baseRevision),
        });
        if (!admitted()) return;
        if (result.status !== 'restored') {
          restoreBlocked = true;
          error ||= 'No se pudo recuperar el borrador. Vuelve a consultar el contexto.';
          return;
        }
      } else register();
    } catch (failure) {
      if (alive) {
        error = failure.message;
        restoreBlocked = true;
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function close() {
    if (busy) return;
    if (editorKey) sessionDrafts?.registry.closeEditor(editorKey);
    registration?.dispose();
    registration = editorKey = null;
    dialog.close();
    draft = metadataDraft();
    fields?.reset();
  }
  async function refresh() {
    if (busy || disabled || !admitted()) return;
    busy = true;
    try {
      if (restored && !(await freshContext())) return;
      const result = await api.get();
      if (!admitted()) return;
      candidate = result;
      if (needsReview) reviewAccepted = false;
      await onconfirmed(result);
      if (!admitted()) return;
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (
      disabled ||
      busy ||
      exhausted ||
      !admitted() ||
      restoredClosed ||
      restoreBlocked ||
      (needsReview && !reviewAccepted) ||
      $administration.closed ||
      (conflict && !candidate)
    )
      return;
    let values;
    try {
      values = fields.values();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      error = failure.field === 'tag' ? '' : failure.message;
      return;
    }
    busy = true;
    error = '';
    unconfirmed = true;
    try {
      const result = await api.replace(candidate?.metadata_revision ?? expected, values);
      if (!admitted()) return;
      await onconfirmed(result);
      if (!admitted()) return;
      unconfirmed = false;
      busy = false;
      close();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (!alive) return;
      unconfirmed = !failure.status;
      const metadataConflict = failure.code === 'document_metadata_conflict';
      conflict = metadataConflict || restored;
      if (restored) {
        needsReview = true;
        reviewAccepted = false;
      }
      exhausted = failure.code === 'document_metadata_revision_exhausted';
      candidate = null;
      error = metadataConflict
        ? 'La clasificaci\u00f3n cambi\u00f3 mientras editabas. Consulta los valores actuales y revisa tus cambios.'
        : mutationError(failure);
      if ([403, 404].includes(failure.status)) deny(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    registration?.dispose();
    busy = false;
  });
</script>

<dialog
  class="upload-dialog metadata-dialog"
  bind:this={dialog}
  aria-labelledby="metadata-edit-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">ORGANIZACI&#211;N DEL DOCUMENTO</span>
      <h2 id="metadata-edit-title">Editar clasificaci&#243;n</h2>
    </div>
    <button
      class="icon-button"
      disabled={busy}
      onclick={close}
      aria-label="Cerrar clasificaci&#243;n"><Icon name="close" /></button
    >
  </div>
  <p>
    Actualiza la organizaci&#243;n del documento. Sus archivos y evidencias conservan sus versiones.
  </p>
  <form class="stack" onsubmit={submit}>
    <MetadataFields
      prefix="edit-metadata"
      bind:this={fields}
      bind:draft
      disabled={busy || restoreBlocked}
    />
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if restoreBlocked}<button type="button" class="secondary" disabled={busy} onclick={open}
        >Volver a consultar el contexto</button
      >{/if}
    {#if conflict || restoredClosed}<button
        type="button"
        class="secondary"
        disabled={disabled || busy}
        onclick={refresh}>Consultar clasificaci&#243;n actual</button
      >{/if}
    {#if candidate}<section class="metadata-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.metadata_revision}</p>
        <MetadataSummary metadata={candidate} />
        <p class="hint">Guardar mis cambios reemplazar&#225; estos valores con tu formulario.</p>
        {#if needsReview}<label class="check-row"
            ><input
              type="checkbox"
              bind:checked={reviewAccepted}
              disabled={busy || restoredClosed || $administration.closed}
            />
            He comparado los valores actuales y decido guardar mis cambios.
          </label>{/if}
      </section>{/if}
    {#if restoredClosed && !$administration.closed}<p class="notice" role="status">
        El expediente est&#225; cerrado administrativamente. Conserva tu borrador y consulta su
        estado antes de guardar.
      </p>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button type="button" class="secondary" disabled={busy} onclick={close}>Cancelar</button>
      <button
        class="primary"
        disabled={disabled ||
          busy ||
          exhausted ||
          restoredClosed ||
          restoreBlocked ||
          (needsReview && !reviewAccepted) ||
          $administration.closed ||
          (conflict && !candidate)}
        >{busy
          ? 'Guardando...'
          : candidate
            ? 'Guardar mis cambios'
            : 'Guardar clasificaci\u00f3n'}</button
      >
    </div>
  </form>
</dialog>
