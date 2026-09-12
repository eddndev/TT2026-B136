<script>
  import Icon from './Icon.svelte';
  import { safeFilename, validateUpload } from '../lib/documents.mjs';
  export let api;
  export let onuploaded;
  let dialog;
  let file = null;
  let name = '';
  let busy = false;
  let error = '';
  let dragging = false;
  let input;
  export function open() {
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
    if (input) input.value = '';
  }
  async function submit(event) {
    event.preventDefault();
    error = validateUpload(file, name);
    if (error) return;
    busy = true;
    try {
      const result = await api.upload(file, name.trim());
      onuploaded(result);
      busy = false;
      close();
    } catch (failure) {
      error = failure.message;
    } finally {
      busy = false;
    }
  }
</script>

<dialog
  class="upload-dialog"
  bind:this={dialog}
  aria-labelledby="upload-title"
  oncancel={(event) => {
    if (busy) event.preventDefault();
    else {
      file = null;
      name = '';
      if (input) input.value = '';
    }
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
      aria-label="Area para arrastrar archivo"
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
        >{file ? file.name : 'Arrastra tu archivo aqui'}</strong
      ><span
        >{file
          ? `${(file.size / 1024).toFixed(1)} KB seleccionados`
          : 'o selecciona uno desde tu equipo'}</span
      ><label
        >Archivo<input
          bind:this={input}
          type="file"
          disabled={busy}
          onchange={(event) => choose(event.currentTarget.files[0])}
        /></label
      ><small>Cualquier formato. Maximo 16 MiB por documento.</small>
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
      Te sugerimos un nombre compatible para que puedas descargar su evidencia despues. Puedes
      editarlo antes de cargar.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <div class="dialog-actions">
      <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button
      ><button class="primary" disabled={busy}
        >{busy ? 'Cargando documento...' : 'Cargar documento'}<Icon
          name="arrow"
          size={17}
        /></button
      >
    </div>
  </form>
</dialog>
