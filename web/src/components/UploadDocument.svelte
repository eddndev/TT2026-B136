<script>
  import { uploadOutcomeUncertain } from '../lib/document-upload-outcome.mjs';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { getContext, onDestroy } from 'svelte';
  import MetadataFields from './MetadataFields.svelte';
  import { metadataDraft, mutationError } from '../lib/document-metadata.mjs';
  import Icon from './Icon.svelte';
  import { safeFilename, validateUpload, uploadFormats } from '../lib/documents.mjs';
  import { createRootUploadDraft } from '../lib/root-upload-draft.mjs';
  export let api;
  export let onuploaded;
  export let ondenied = () => {};
  export let draftContext = null;
  let dialog;
  let fields;
  let draft = metadataDraft();
  let alive = true;
  let file = null;
  let name = '';
  export let busy = false,
    disabled = false;
  let error = '';
  let restoreBlocked = false,
    restoredClosed = false,
    unconfirmed = false,
    reviewAccepted = false,
    reviewComplete = false,
    reviewStarted = false,
    reviewedNames = [],
    reviewOffset = 0;
  const sessionDrafts = getContext('session-drafts');
  const recovery =
    draftContext && sessionDrafts
      ? createRootUploadDraft({
          session: sessionDrafts,
          caseId: draftContext.caseId,
          owner: draftContext.ownerDraftKey ? draftContext : null,
          capture: () => ({ file, name, draft, rawFields: fields.captureDraft(), unconfirmed }),
        })
      : null;
  const admitted = () => (recovery ? recovery.admitted() : alive);
  $: fieldsBlocked = busy || restoreBlocked || restoredClosed;
  $: submitBlocked =
    fieldsBlocked || disabled || $administration.closed || (unconfirmed && !reviewAccepted);
  let blockedByCase = false;
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
    restoredClosed = false;
  }
  let dragging = false;
  let input;
  export function hasSuspendedDraft() {
    return recovery?.pending() ?? false;
  }
  export function open() {
    if (recovery) return openRoot();
    if ($administration.closed || disabled) return;
    error = '';
    dialog.showModal();
  }
  function resetReview() {
    reviewAccepted = reviewComplete = reviewStarted = false;
    reviewedNames = [];
    reviewOffset = 0;
  }
  function deny(failure) {
    if (![403, 404].includes(failure.status)) return;
    recovery?.discard(failure);
    if (recovery) restoreBlocked = true;
    ondenied(failure);
  }
  async function openRoot() {
    if (busy || disabled || !admitted() || (dialog.open && !restoreBlocked)) return;
    const saved = recovery.pending();
    if ($administration.closed && !saved) return;
    restoreBlocked = true;
    restoredClosed = false;
    resetReview();
    error = '';
    if (!dialog.open) dialog.showModal();
    busy = true;
    try {
      if (saved) {
        const result = await recovery.restore((value, status) => {
          fields.restoreDraft(value.rawFields);
          ({ file, name, draft, unconfirmed } = value);
          restoredClosed = status === 'closed';
          restoreBlocked = false;
        });
        if (admitted() && result.status !== 'restored')
          error = 'No se pudo recuperar el borrador. Vuelve a consultar el expediente.';
      } else {
        const status = await recovery.freshContext();
        if (!admitted() || status === null) return;
        restoredClosed = status === 'closed';
        if (restoredClosed) return;
        recovery.register();
        restoreBlocked = false;
      }
    } catch (failure) {
      if (alive) {
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function choose(next) {
    file = next || null;
    name = safeFilename(file?.name || '');
    error = '';
  }
  function close() {
    if (busy) return;
    recovery?.close();
    dialog.close();
    file = null;
    name = '';
    draft = metadataDraft();
    restoreBlocked = restoredClosed = unconfirmed = false;
    resetReview();
    fields?.reset();
    if (input) input.value = '';
  }
  async function reviewDocuments() {
    if (busy || !admitted() || restoreBlocked || !unconfirmed) return;
    busy = true;
    reviewAccepted = false;
    error = '';
    try {
      const status = await recovery.freshContext();
      if (!admitted() || status === null) return;
      restoredClosed = status === 'closed';
      const result = await api.list({ limit: 50, offset: reviewOffset });
      if (!admitted()) return;
      if (
        !Array.isArray(result.documents) ||
        typeof result.has_more !== 'boolean' ||
        result.documents.some(
          (item) => item.case_id !== draftContext.caseId || typeof item.name !== 'string',
        ) ||
        (result.has_more && !result.documents.length)
      )
        throw new Error('No se pudo confirmar el listado actual.');
      reviewedNames = [...reviewedNames, ...result.documents.map((item) => item.name)];
      reviewOffset += result.documents.length;
      reviewStarted = true;
      reviewComplete = !result.has_more;
    } catch (failure) {
      if (alive) {
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || submitBlocked || !admitted()) return;
    error = validateUpload(file, name);
    if (error) return;
    let metadata;
    try {
      metadata = fields.values();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      error = failure.field === 'tag' ? '' : failure.message;
      return;
    }
    busy = true;
    if (recovery) {
      unconfirmed = true;
      resetReview();
    }
    try {
      const result = await api.uploadWithMetadata(file, name.trim(), metadata);
      if (!admitted()) return;
      onuploaded(result);
      busy = false;
      close();
    } catch (failure) {
      if (!alive) return;
      if (failure.code === 'case_closed') blockedByCase = true;
      if (recovery && failure.code === 'case_closed') restoredClosed = true;
      if (recovery) unconfirmed = uploadOutcomeUncertain(failure);
      deny(failure);
      error = mutationError(failure, 'cargar');
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
  });
</script>

<dialog
  class="upload-dialog"
  bind:this={dialog}
  aria-labelledby="upload-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">NUEVO ARCHIVO</span>
      <h2 id="upload-title">Subir documento</h2>
    </div>
    <button class="icon-button" disabled={busy} onclick={close} aria-label="Cerrar carga"
      ><Icon name="close" /></button
    >
  </div>
  <p>Agrega un archivo para comenzar su registro y proteger su evidencia.</p>
  <form class="stack" onsubmit={submit}>
    <div
      class="file-drop"
      class:dragging
      role="region"
      aria-label="&#193;rea para arrastrar archivo"
      ondragover={(event) => {
        event.preventDefault();
        dragging = true;
      }}
      ondragleave={() => (dragging = false)}
      ondrop={(event) => {
        event.preventDefault();
        dragging = false;
        if (!fieldsBlocked) choose(event.dataTransfer.files[0]);
      }}
    >
      <span class="tile-icon"><Icon name="upload" size={28} /></span><strong
        >{file ? file.name : 'Arrastra tu archivo aqu\u00ed'}</strong
      ><span
        >{file
          ? `${(file.size / 1024).toFixed(1)} KiB seleccionados`
          : 'o selecciona uno desde tu equipo'}</span
      ><label
        >Archivo<input
          bind:this={input}
          type="file"
          disabled={fieldsBlocked}
          onchange={(event) => choose(event.currentTarget.files[0])}
        /></label
      ><small
        >Formatos admitidos: {uploadFormats}. M&#225;ximo: 16 MiB por documento. El contenido se
        valida antes de guardarlo.</small
      >
    </div>
    <label
      >Nombre del documento<input
        required
        maxlength="124"
        disabled={fieldsBlocked}
        bind:value={name}
        placeholder="contrato.pdf"
      /></label
    >
    <p class="hint">
      Te sugerimos un nombre compatible para que puedas descargar su evidencia despu&#233;s. Puedes
      editarlo antes de cargar.
    </p>
    <MetadataFields
      prefix="upload-metadata"
      bind:this={fields}
      bind:draft
      disabled={fieldsBlocked}
    />
    {#if restoreBlocked && !restoredClosed}<p class="notice" role="status">
        Confirma el expediente actual antes de recuperar o editar el borrador.
      </p>
      <button class="secondary" type="button" disabled={busy} onclick={openRoot}
        >Volver a consultar el expediente</button
      >{/if}
    {#if restoredClosed}<p class="notice" role="status">
        El expediente est&#225; cerrado. Puedes consultar el borrador, pero no cargarlo.
      </p>{/if}
    {#if unconfirmed && !restoreBlocked}<section class="notice" aria-label="Carga sin confirmar">
        <p role="status">
          No se pudo confirmar la carga anterior. Consulta los documentos actuales antes de decidir
          si iniciar otra carga.
        </p>
        {#if !reviewComplete}<button
            class="secondary"
            type="button"
            disabled={busy}
            onclick={reviewDocuments}
            >{reviewStarted
              ? 'Consultar siguientes documentos'
              : 'Consultar documentos actuales'}</button
          >{/if}
        {#if reviewStarted}<ul aria-label="Documentos actuales">
            {#each reviewedNames as documentName}<li>{documentName}</li>{/each}
          </ul>{/if}
        {#if reviewComplete}<label
            ><input
              type="checkbox"
              bind:checked={reviewAccepted}
              disabled={busy || restoredClosed}
            />He revisado el listado y decido iniciar otra carga; puede duplicar la anterior.</label
          >{/if}
      </section>{/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button
      ><button class="primary" disabled={submitBlocked}
        >{busy ? 'Cargando documento...' : 'Cargar documento'}<Icon
          name="arrow"
          size={17}
        /></button
      >
    </div>
  </form>
</dialog>
