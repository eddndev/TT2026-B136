<script>
  import { onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { safeFilename, validateUpload } from '../lib/documents.mjs';
  export let api;
  export let document;
  export let onappended;
  export let oncurrent;
  export let ondenied = () => {};
  let dialog;
  let file = null;
  let name = '';
  let expectedVersion = document.version;
  let busy = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let input;
  let alive = true;
  export function open() {
    expectedVersion = document.version;
    error = '';
    conflict = false;
    exhausted = false;
    dialog.showModal();
  }
  function choose(next) {
    file = next || null;
    name = safeFilename(file?.name || '');
  }
  function close() {
    if (busy) return;
    dialog.close();
    file = null;
    name = '';
    if (input) input.value = '';
  }
  async function refresh() {
    busy = true;
    error = '';
    try {
      const latest = await api.detail(document.id);
      if (!alive) return;
      expectedVersion = latest.version;
      oncurrent(latest);
      conflict = false;
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
    if (busy || conflict || exhausted) return;
    error = validateUpload(file, name);
    if (error) return;
    busy = true;
    try {
      const result = await api.append(document.id, expectedVersion, file, name.trim());
      if (!alive) return;
      onappended(result);
      busy = false;
      close();
    } catch (failure) {
      if (!alive) return;
      if ([403, 404].includes(failure.status)) ondenied(failure);
      conflict = failure.code === 'document_version_conflict';
      exhausted = failure.code === 'document_version_exhausted';
      error = conflict
        ? 'El documento cambi\u00f3 mientras preparabas el archivo. Consulta la versi\u00f3n actual, revisa tu archivo y confirma de nuevo.'
        : failure.message;
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
  aria-labelledby="append-version-title"
  oncancel={(event) => {
    event.preventDefault();
    close();
  }}
>
  <div class="dialog-heading">
    <div>
      <span class="eyebrow">HISTORIAL DOCUMENTAL</span>
      <h2 id="append-version-title">Agregar versi&#243;n</h2>
    </div>
    <button
      class="icon-button"
      disabled={busy}
      onclick={close}
      aria-label="Cerrar nueva versi&#243;n"><Icon name="close" /></button
    >
  </div>
  <p>
    Agrega un archivo al mismo documento. Las versiones anteriores y su evidencia permanecen
    disponibles.
  </p>
  <p class="version-origin">Versi&#243;n de partida: {expectedVersion}</p>
  <form class="stack" onsubmit={submit}>
    <div
      class="file-drop"
      role="region"
      aria-label="Archivo para la nueva versi&#243;n"
      ondragover={(event) => event.preventDefault()}
      ondrop={(event) => {
        event.preventDefault();
        if (!busy) choose(event.dataTransfer.files[0]);
      }}
    >
      <span class="tile-icon"><Icon name="upload" size={28} /></span><strong
        >{file ? file.name : 'Selecciona o arrastra el archivo'}</strong
      >
      <label
        >Archivo de la nueva versi&#243;n<input
          type="file"
          bind:this={input}
          disabled={busy}
          onchange={(event) => choose(event.currentTarget.files[0])}
        /></label
      ><small>M&#225;ximo: 16 MiB por versi&#243;n.</small>
    </div>
    <label
      >Nombre de la nueva versi&#243;n<input
        required
        maxlength="124"
        bind:value={name}
        disabled={busy}
        placeholder="documento-actualizado.pdf"
      /></label
    >
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if conflict}<button class="secondary" type="button" disabled={busy} onclick={refresh}
        >Consultar versi&#243;n actual</button
      >{/if}
    <div class="dialog-actions">
      <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button
      ><button class="primary" disabled={busy || conflict || exhausted}
        >{busy ? 'Guardando versi\u00f3n...' : 'Guardar nueva versi\u00f3n'}<Icon
          name="arrow"
          size={17}
        /></button
      >
    </div>
  </form>
</dialog>
