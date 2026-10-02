<script>
  import { uploadOutcomeUncertain } from '../lib/document-upload-outcome.mjs';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import { getContext, onDestroy } from 'svelte';
  import Icon from './Icon.svelte';
  import { safeFilename, validateUpload, uploadFormats } from '../lib/documents.mjs';
  import {
    createAppendVersionDraft,
    validateAppendDocument,
  } from '../lib/append-version-draft.mjs';
  export let api;
  export let document;
  export let onappended;
  export let oncurrent;
  export let ondenied = () => {};
  export let disabled = false;
  let dialog;
  let file = null;
  let name = '';
  let expectedVersion = document.version;
  let originalExpectedVersion = expectedVersion;
  export let busy = false;
  let conflict = false;
  let exhausted = false;
  let error = '';
  let blockedByCase = false;
  let restoring = false,
    restored = false,
    restoreBlocked = false,
    restoredClosed = false,
    unconfirmed = false,
    needsReview = false,
    reviewAccepted = false,
    candidate = null;
  const sessionDrafts = getContext('session-drafts');
  const recovery = sessionDrafts
    ? createAppendVersionDraft({
        session: sessionDrafts,
        caseId: document.case_id,
        documentId: document.id,
        readDocument: () => api.detail(document.id),
        capture: () => ({ file, name, originalExpectedVersion, unconfirmed }),
      })
    : null;
  const admitted = () => (recovery ? recovery.admitted() : alive);
  $: fieldsBlocked = busy || restoreBlocked || restoredClosed || $administration.closed;
  $: submitBlocked =
    fieldsBlocked ||
    disabled ||
    exhausted ||
    $administration.closed ||
    (conflict && !candidate) ||
    (needsReview && !reviewAccepted);
  $: if (blockedByCase && !$administration.closed) {
    error = '';
    blockedByCase = false;
  }
  let input;
  let alive = true;
  export function hasSuspendedDraft() {
    return recovery?.pending() ?? false;
  }
  export function discardDraft(failure) {
    return recovery?.discard(failure);
  }
  function deny(failure) {
    if (![403, 404].includes(failure.status)) return;
    discardDraft(failure);
    restoreBlocked = true;
    ondenied(failure);
  }
  export async function open() {
    if (busy || disabled || !admitted() || (dialog.open && !restoreBlocked)) return;
    const saved = recovery?.pending();
    if ($administration.closed && !saved) return;
    originalExpectedVersion = expectedVersion = document.version;
    error = '';
    conflict = false;
    exhausted = false;
    restored = restoredClosed = unconfirmed = needsReview = reviewAccepted = false;
    candidate = null;
    restoreBlocked = !!saved;
    if (!dialog.open) dialog.showModal();
    busy = restoring = true;
    try {
      if (saved) {
        let latest;
        const result = await recovery.restore((value, current) => {
          ({ file, name, originalExpectedVersion, unconfirmed } = value);
          expectedVersion = originalExpectedVersion;
          restored = true;
          restoredClosed = current.status === 'closed';
          latest = current.document;
          needsReview = latest.version !== originalExpectedVersion || unconfirmed;
          candidate = needsReview || restoredClosed ? latest : null;
          conflict = needsReview;
          restoreBlocked = false;
        });
        if (!admitted()) return;
        if (result.status !== 'restored') {
          error = 'No se pudo recuperar el borrador. Vuelve a consultar el contexto.';
          return;
        }
        await oncurrent(latest);
        if (!admitted()) return;
      } else recovery?.register(originalExpectedVersion);
    } catch (failure) {
      if (alive) {
        if (!restored) restoreBlocked = true;
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = restoring = false;
    }
  }
  function choose(next) {
    if (busy || fieldsBlocked || !admitted()) return;
    file = next || null;
    name = safeFilename(file?.name || '');
    if (needsReview) reviewAccepted = false;
  }
  function close() {
    if (busy) return;
    recovery?.close();
    dialog.close();
    file = null;
    name = '';
    restored = restoreBlocked = restoredClosed = unconfirmed = needsReview = reviewAccepted = false;
    candidate = null;
    if (input) input.value = '';
  }
  async function refresh() {
    if (busy || disabled || restoreBlocked || !admitted()) return;
    busy = restoring = true;
    error = '';
    if (restored || unconfirmed) {
      candidate = null;
      needsReview = true;
      reviewAccepted = false;
    }
    try {
      const current =
        recovery && (restored || unconfirmed)
          ? await recovery.freshContext()
          : {
              document: await api.detail(document.id),
              status: $administration.closed ? 'closed' : 'active',
            };
      if (!current || !admitted()) return;
      const latest = current.document;
      validateAppendDocument(latest, document.case_id, document.id);
      if (restored && latest.version < originalExpectedVersion)
        throw new Error('La versi\u00f3n actual no confirma la base del borrador.');
      restoredClosed = current.status === 'closed';
      if (restored || unconfirmed) {
        candidate = latest;
        needsReview = true;
        reviewAccepted = false;
      } else expectedVersion = latest.version;
      await oncurrent(latest);
      if (!admitted()) return;
      conflict = false;
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (alive) {
        error = failure.message;
        deny(failure);
      }
    } finally {
      if (alive) busy = restoring = false;
    }
  }
  async function submit(event) {
    event.preventDefault();
    if (busy || submitBlocked || !admitted()) return;
    error = validateUpload(file, name);
    if (error) return;
    busy = true;
    unconfirmed = true;
    reviewAccepted = false;
    try {
      const result = await api.append(
        document.id,
        candidate?.version ?? expectedVersion,
        file,
        name.trim(),
      );
      if (!admitted()) return;
      unconfirmed = false;
      recovery?.close();
      onappended(result);
      busy = false;
      close();
    } catch (failure) {
      if (failure.code === 'case_closed') blockedByCase = true;
      if (!alive) return;
      if (failure.code === 'case_closed') restoredClosed = true;
      unconfirmed = uploadOutcomeUncertain(failure);
      deny(failure);
      const versionConflict = failure.code === 'document_version_conflict';
      conflict = versionConflict || restored;
      if (restored || unconfirmed) needsReview = true;
      candidate = null;
      exhausted = failure.code === 'document_version_exhausted';
      error = versionConflict
        ? 'El documento cambi\u00f3 mientras preparabas el archivo. Consulta la versi\u00f3n actual, revisa tu archivo y confirma de nuevo.'
        : failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  onDestroy(() => {
    alive = false;
    recovery?.dispose();
    busy = false;
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
        if (!fieldsBlocked) choose(event.dataTransfer.files[0]);
      }}
    >
      <span class="tile-icon"><Icon name="upload" size={28} /></span><strong
        >{file ? file.name : 'Selecciona o arrastra el archivo'}</strong
      >
      <label
        >Archivo de la nueva versi&#243;n<input
          type="file"
          bind:this={input}
          disabled={fieldsBlocked}
          onchange={(event) => choose(event.currentTarget.files[0])}
        /></label
      ><small
        >Formatos admitidos: {uploadFormats}. M&#225;ximo: 16 MiB por versi&#243;n. El contenido se
        valida antes de guardarlo.</small
      >
    </div>
    <label
      >Nombre de la nueva versi&#243;n<input
        required
        maxlength="124"
        bind:value={name}
        disabled={fieldsBlocked}
        placeholder="documento-actualizado.pdf"
      /></label
    >
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    {#if restoreBlocked}<p class="notice" role="status">
        Confirma el expediente y el documento actuales antes de recuperar el borrador.
      </p>
      <button class="secondary" type="button" disabled={busy} onclick={open}
        >Volver a consultar el contexto</button
      >{/if}
    {#if unconfirmed && !restoreBlocked}<p class="notice" role="status">
        No se pudo confirmar la versi&#243;n enviada. Revisa la versi&#243;n actual antes de decidir
        si agregar otra.
      </p>{/if}
    {#if conflict || unconfirmed || restoredClosed}<button
        class="secondary"
        type="button"
        disabled={disabled || busy || restoreBlocked}
        onclick={refresh}>Consultar versi&#243;n actual</button
      >{/if}
    {#if candidate}<section class="metadata-comparison" aria-label="Versi&#243;n actual guardada">
        <h3>Versi&#243;n actual guardada</h3>
        <p>Versi&#243;n {candidate.version} / {candidate.name}</p>
        <p class="hint">El nuevo archivo se agregar&#225; despu&#233;s de esta versi&#243;n.</p>
        {#if needsReview}<label class="check-row"
            ><input
              type="checkbox"
              bind:checked={reviewAccepted}
              disabled={fieldsBlocked || disabled || $administration.closed}
            />
            He comparado la versi&#243;n actual y decido agregar mi archivo.
          </label>{/if}
      </section>{/if}
    {#if restoredClosed}<p class="notice" role="status">
        El expediente est&#225; cerrado. Puedes consultar el borrador, pero no agregar su archivo.
      </p>{/if}
    <CaseClosedNotice />
    <div class="dialog-actions">
      <button class="secondary" type="button" disabled={busy} onclick={close}>Cancelar</button
      ><button class="primary" disabled={submitBlocked}
        >{busy && !restoring ? 'Guardando versi\u00f3n...' : 'Guardar nueva versi\u00f3n'}<Icon
          name="arrow"
          size={17}
        /></button
      >
    </div>
  </form>
</dialog>
