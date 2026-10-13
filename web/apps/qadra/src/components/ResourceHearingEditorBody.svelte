<script>
  import ResourceHearingFields from './ResourceHearingFields.svelte';
  import ResourceHearingReview from './ResourceHearingReview.svelte';
  import ResourceValues from './ResourceValues.svelte';
  export let api,
    caseId,
    resource,
    act,
    fields,
    participants,
    ondenied,
    canApply,
    inputs,
    pending,
    fieldsBusy,
    disabled,
    frozen,
    blocked,
    closed,
    mode,
    error,
    prepared,
    acknowledged,
    candidate,
    retryAvailable,
    actions,
    initialize,
    close;
  let form;
  export function captureInputs() {
    return form?.captureInputs() ?? inputs;
  }
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Formulario de audiencia de recurso"
  aria-busy={pending}
>
  <h2>Crear audiencia de recurso</h2>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if closed}<p class="notice">
      Expediente cerrado administrativamente. Consulta el resultado historico; no se admite una
      nueva programacion.
    </p>{/if}
  {#if blocked || closed}<button
      class="secondary"
      disabled={pending || disabled}
      onclick={initialize}>Volver a consultar el contexto</button
    >{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>Consulta la audiencia exacta y su asociaci&#243;n original antes de decidir.</p>
    <button class="primary" disabled={pending || disabled || blocked} onclick={actions.check}
      >Consultar resultado</button
    >
    {#if retryAvailable}<button
        class="secondary"
        disabled={pending || disabled || blocked || closed}
        onclick={actions.retry}>Reintentar envio exacto</button
      >{/if}
    <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
  {:else if mode === 'conflict'}
    <h3>El registro cambi&#243;</h3>
    <p>La programaci&#243;n y las revisiones hist&#243;ricas seleccionadas se conservan.</p>
    <button class="secondary" disabled={pending || disabled || blocked} onclick={actions.compare}
      >Comparar con registro actual</button
    >
    {#if candidate}
      <p>
        Cabeza actual del recurso: revisi&#243;n {candidate.revision} / {candidate.status ===
        'active'
          ? 'Activo'
          : 'Archivado'}
      </p>
      <ResourceValues values={candidate.values} />
      <button
        class="primary"
        disabled={pending || disabled || blocked || closed || candidate.status !== 'active'}
        onclick={actions.accept}>Usar base actual y conservar borrador</button
      >
    {/if}
  {:else if mode === 'review' && prepared}
    <ResourceHearingReview value={prepared} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Reconozco la
      programacion y las capturas seleccionadas</label
    >
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar audiencia y vinculo</button
      >
      <button
        class="secondary"
        disabled={frozen}
        onclick={() => {
          mode = 'draft';
          prepared = null;
          acknowledged = false;
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if blocked}
    <p role="status">Confirma tu acceso actual antes de recuperar las capturas.</p>
    <button class="primary" disabled>Preparar audiencia y vinculo</button>
  {:else if !['review', 'confirmed'].includes(mode)}
    <ResourceHearingFields
      {api}
      {caseId}
      {resource}
      {ondenied}
      {inputs}
      {canApply}
      bind:this={form}
      bind:fields
      bind:act
      bind:participants
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={actions.prepare}
      >Preparar audiencia y vinculo</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={close}
    >Cerrar formulario</button
  >
</section>
