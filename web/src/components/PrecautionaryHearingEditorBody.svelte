<script>
  import PrecautionaryHearingFields from './PrecautionaryHearingFields.svelte';
  import PrecautionaryHearingReview from './PrecautionaryHearingReview.svelte';
  export let api,
    caseId,
    action,
    fields,
    participants,
    support,
    reviewTargets,
    inputs,
    ondenied,
    canApply,
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
    retryAvailable,
    actions,
    initialize,
    close;
  let form;
  $: title =
    action === 'schedule'
      ? 'Programar audiencia cautelar'
      : action === 'replace'
        ? 'Reprogramar audiencia cautelar'
        : 'Cancelar audiencia cautelar';
  export function captureInputs() {
    return form?.captureInputs() ?? inputs;
  }
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Convocatoria cautelar"
  aria-busy={pending}
>
  <h2>{title}</h2>
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if closed}<p class="notice">
      Expediente cerrado administrativamente. La consulta del resultado sigue disponible. La
      convocatoria no admite cambios nuevos.
    </p>{/if}
  {#if blocked || closed}<button
      class="secondary"
      disabled={pending || disabled}
      onclick={initialize}>Volver a consultar el contexto</button
    >{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>Consulta el resultado de esta operaci&#243;n antes de decidir un reintento.</p>
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
    <h3>La convocatoria o su contexto cambi&#243;</h3>
    <p>El borrador conserva su base y las referencias seleccionadas.</p>
    {#if !blocked && !closed}<button
        class="secondary"
        disabled={pending || disabled}
        onclick={initialize}>Volver a consultar el contexto</button
      >{/if}
  {:else if mode === 'review' && prepared}
    <PrecautionaryHearingReview value={prepared} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Reconozco la
      convocatoria y las fuentes seleccionadas</label
    >
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar convocatoria</button
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
    <p role="status">Confirma tu acceso actual antes de recuperar la convocatoria.</p>
    <button class="primary" disabled>Revisar convocatoria</button>
  {:else if !['review', 'confirmed'].includes(mode)}
    <PrecautionaryHearingFields
      {api}
      {caseId}
      {action}
      {inputs}
      {ondenied}
      {canApply}
      bind:this={form}
      bind:fields
      bind:participants
      bind:support
      bind:reviewTargets
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={actions.prepare}
      >Revisar convocatoria</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={close}
    >Cerrar formulario</button
  >
</section>
