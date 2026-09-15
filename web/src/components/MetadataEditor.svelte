<script>
  import { onDestroy } from 'svelte';
  import MetadataFields from './MetadataFields.svelte';
  import MetadataSummary from './MetadataSummary.svelte';
  import Icon from './Icon.svelte';
  import { metadataDraft, mutationError } from '../lib/document-metadata.mjs';
  export let api;
  export let current;
  export let onconfirmed;
  export let ondenied;
  let dialog;
  let fields;
  let draft = metadataDraft();
  let expected = 0;
  let candidate = null;
  let busy = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let alive = true;
  export function open() {
    draft = metadataDraft(current);
    expected = current.metadata_revision;
    candidate = null;
    conflict = false;
    exhausted = false;
    error = '';
    fields?.reset();
    dialog.showModal();
  }
  function close() {
    if (busy) return;
    dialog.close();
    draft = metadataDraft();
    fields?.reset();
  }
  async function refresh() {
    if (busy) return;
    busy = true;
    try {
      const result = await api.get();
      if (!alive) return;
      candidate = result;
      onconfirmed(result);
    } catch (failure) {
      if (alive) {
        error = failure.message;
        if ([403, 404].includes(failure.status)) ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || exhausted || (conflict && !candidate)) return;
    let values;
    try {
      values = fields.values();
    } catch (failure) {
      error = failure.field === 'tag' ? '' : failure.message;
      return;
    }
    busy = true;
    error = '';
    try {
      const result = await api.replace(candidate?.metadata_revision ?? expected, values);
      if (!alive) return;
      onconfirmed(result);
      busy = false;
      close();
    } catch (failure) {
      if (!alive) return;
      conflict = failure.code === 'document_metadata_conflict';
      exhausted = failure.code === 'document_metadata_revision_exhausted';
      candidate = null;
      error = conflict
        ? 'La clasificaci\u00f3n cambi\u00f3 mientras editabas. Consulta los valores actuales y revisa tus cambios.'
        : mutationError(failure);
      if ([403, 404].includes(failure.status)) ondenied(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
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
    <MetadataFields prefix="edit-metadata" bind:this={fields} bind:draft disabled={busy} />
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if conflict}<button type="button" class="secondary" disabled={busy} onclick={refresh}
        >Consultar clasificaci&#243;n actual</button
      >{/if}
    {#if candidate}<section class="metadata-comparison" aria-label="Valores actuales guardados">
        <h3>Valores actuales guardados</h3>
        <p class="hint">Revisi&#243;n {candidate.metadata_revision}</p>
        <MetadataSummary metadata={candidate} />
        <p class="hint">Guardar mis cambios reemplazar&#225; estos valores con tu formulario.</p>
      </section>{/if}
    <div class="dialog-actions">
      <button type="button" class="secondary" disabled={busy} onclick={close}>Cancelar</button>
      <button class="primary" disabled={busy || exhausted || (conflict && !candidate)}
        >{busy
          ? 'Guardando...'
          : candidate
            ? 'Guardar mis cambios'
            : 'Guardar clasificaci\u00f3n'}</button
      >
    </div>
  </form>
</dialog>
