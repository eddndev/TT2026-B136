<script>
  import DeadlineFields from './DeadlineFields.svelte';
  import ResourceDeadlineContext from './ResourceDeadlineContext.svelte';
  import ResourceDeadlineReview from './ResourceDeadlineReview.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let api,
    caseId,
    resource,
    ondenied,
    selection,
    definition,
    policies,
    profile,
    responsible,
    fieldsBusy,
    contextBusy,
    mode,
    prepared,
    acknowledged,
    pending,
    disabled,
    frozen,
    closed,
    blocked,
    error,
    candidate,
    retryAvailable,
    paired,
    inputs,
    recoverable,
    actions,
    close,
    initialize;
  let fields, context;
  export function captureInputs() {
    return {
      fields: fields?.captureInputs() ?? inputs?.fields ?? null,
      context: context?.captureInputs() ?? inputs?.context ?? null,
    };
  }
</script>

<section class="card case-editor fact-editor" aria-label="Formulario de plazo" aria-busy={pending}>
  <h2>Crear plazo desde el recurso</h2>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if blocked || closed}<button
      class="secondary"
      disabled={pending || disabled}
      onclick={initialize}>Volver a consultar el contexto</button
    >{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>Consulta las revisiones exactas del plazo y del v&#237;nculo antes de decidir.</p>
    <button class="primary" disabled={pending || disabled || blocked} onclick={actions.check}
      >Consultar resultado</button
    >
    <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    {#if retryAvailable}<button
        class="secondary"
        disabled={pending || disabled || blocked || (closed && !paired)}
        onclick={actions.retry}
        >{paired ? 'Confirmar origen del env\u00edo' : 'Reintentar envio exacto'}</button
      >{/if}
  {:else if mode === 'conflict'}
    <h3>El registro cambi&#243;</h3>
    <p>La declaraci&#243;n del plazo y las capturas seleccionadas se conservan.</p>
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
        disabled={frozen || candidate.status !== 'active'}
        onclick={actions.accept}>Usar base actual y conservar borrador</button
      >
    {/if}
  {:else if mode === 'review' && prepared}
    <ResourceDeadlineReview value={prepared} {profile} />
    <label class="checkbox"
      ><input type="checkbox" bind:checked={acknowledged} disabled={frozen} />Reconozco el resultado
      y las capturas seleccionadas</label
    >
    <div class="action-row">
      <button class="primary" disabled={frozen || !acknowledged} onclick={actions.submit}
        >Confirmar plazo y vinculo</button
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
    <label>T&#237;tulo del plazo<input disabled value="" /></label>
    <button class="primary" disabled>Preparar plazo y vinculo</button>
  {:else if mode !== 'review' && mode !== 'confirmed'}
    <ResourceDeadlineContext
      bind:this={context}
      savedInputs={inputs?.context ?? null}
      {api}
      {caseId}
      {resource}
      {ondenied}
      bind:selection
      bind:busy={contextBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <DeadlineFields
      bind:this={fields}
      savedInputs={inputs?.fields ?? null}
      {recoverable}
      {api}
      {caseId}
      {ondenied}
      bind:value={definition}
      bind:profile
      bind:responsible
      bind:policies
      bind:pending={fieldsBusy}
      disabled={frozen || mode !== 'draft'}
    />
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={actions.prepare}
      >Preparar plazo y vinculo</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled} onclick={close}
    >Cerrar formulario</button
  >
</section>
