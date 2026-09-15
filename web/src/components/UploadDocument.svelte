<script>
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { onDestroy } from 'svelte';
  import MetadataFields from './MetadataFields.svelte';
  import { metadataDraft, mutationError } from '../lib/document-metadata.mjs';
  import Icon from './Icon.svelte';
  import { safeFilename, validateUpload } from '../lib/documents.mjs';
  export let api;
  export let onuploaded;
  export let ondenied = () => {};
  let dialog;
  let fields;
  let draft = metadataDraft();
  let alive = true;
  let file = null;
  let name = '';
  export let busy = false,
    disabled = false;
  let error = '';
  let blockedByCase = false;
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
  }
  let dragging = false;
  let input;
  export function open() {
    if ($administration.closed || disabled) return;
    error = '';
    dialog.showModal();
  }
  function choose(next) {
    file = next || null;
    name = safeFilename(file?.name || '');
    error = '';
  }
  function close() {
    if (busy) return;
    dialog.close();
    file = null;
    name = '';
    draft = metadataDraft();
    fields?.reset();
    if (input) input.value = '';
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || disabled || $administration.closed) return;
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
    try {
      const result = await api.uploadWithMetadata(file, name.trim(), metadata);
      if (!alive) return;
      onuploaded(result);
      busy = false;
      close();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (alive && [403, 404].includes(failure.status)) ondenied(failure);
      if (alive) error = mutationError(failure, 'cargar');
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
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
        if (!busy) choose(event.dataTransfer.files[0]);
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
          disabled={busy}
          onchange={(event) => choose(event.currentTarget.files[0])}
        /></label
      ><small>Cualquier formato. M&#225;ximo: 16 MiB por documento.</small>
    </div>
    <label
      >Nombre del documento<input
        required
        maxlength="124"
        disabled={busy}
        bind:value={name}
        placeholder="contrato.pdf"
      /></label
    >
    <p class="hint">
      Te sugerimos un nombre compatible para que puedas descargar su evidencia despu&#233;s. Puedes
      editarlo antes de cargar.
    </p>
    <MetadataFields prefix="upload-metadata" bind:this={fields} bind:draft disabled={busy} />
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button
      ><button class="primary" disabled={busy || disabled || $administration.closed}
        >{busy ? 'Cargando documento...' : 'Cargar documento'}<Icon
          name="arrow"
          size={17}
        /></button
      >
    </div>
  </form>
</dialog>
