<script>
  import ResourceActivityFields from './ResourceActivityFields.svelte';
  import ResourceActivitySources from './ResourceActivitySources.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let api,
    caseId,
    resource,
    selection,
    ondenied,
    fieldsBusy,
    action,
    reason,
    pending,
    disabled,
    frozen,
    mode,
    prepared,
    last,
    candidate,
    retryAvailable,
    error,
    closed,
    blocked,
    inputs,
    fields = null,
    check,
    retry,
    compare,
    accept,
    submit,
    prepare,
    close,
    initialize;
  export function captureInputs() {
    return fields?.captureInputs() ?? inputs;
  }
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Formulario de actividad vinculada"
  aria-busy={pending}
>
  <h2>{action === 'link' ? 'Vincular actividad existente' : 'Desvincular actividad'}</h2>
  <p class="hint">
    Organiza el recurso y conserva la evidencia exacta. No crea ni cancela audiencias, plazos o
    alertas.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if blocked || closed}<button
      class="secondary"
      disabled={pending || disabled}
      onclick={initialize}>Volver a consultar el contexto</button
    >{/if}
  {#if mode === 'uncertain'}
    <h3>Resultado incierto</h3>
    <p>
      Consulta el recibo exacto antes de decidir. Una ausencia temporal no confirma que el
      env&#237;o fall&#243;.
    </p>
    <button class="primary" disabled={pending || disabled || blocked} onclick={check}
      >Consultar resultado</button
    >
    {#if retryAvailable}
      <p>
        La consulta no encontr&#243; la revisi&#243;n. Puedes repetir el mismo env&#237;o con su
        recibo, sin cambiar el borrador.
      </p>
      <button class="secondary" disabled={pending || disabled || closed || blocked} onclick={retry}
        >Reintentar envio exacto</button
      >
    {/if}
  {:else if mode === 'conflict'}
    <h3>El registro cambi&#243;</h3>
    <p>Tu selecci&#243;n exacta y el motivo se conservan.</p>
    <button class="secondary" disabled={pending || disabled || blocked} onclick={compare}
      >Comparar con registro actual</button
    >
    {#if candidate}
      <p>
        Cabeza actual del recurso: revisi&#243;n {candidate.resource.revision} / {candidate.resource
          .status === 'active'
          ? 'Activo'
          : 'Archivado'}
      </p>
      <ResourceValues values={candidate.resource.values} />
      {#if candidate.association}<p>
          V&#237;nculo: {candidate.association.status === 'linked' ? 'Vinculado' : 'Desvinculado'} / Revisi&#243;n
          {candidate.association.revision}
        </p>{/if}
      <button
        class="primary"
        disabled={frozen ||
          (action === 'link'
            ? candidate.resource.status !== 'active'
            : candidate.association.status !== 'linked')}
        onclick={accept}>Usar base actual y conservar borrador</button
      >
    {/if}
  {:else if mode === 'review' && prepared}
    <h3>Revisar antes de confirmar</h3>
    <p>Cabeza del recurso al preparar: revisi&#243;n {prepared.observed_resource_head.revision}</p>
    <ResourceActivitySources sources={prepared.sources} />
    {#if action === 'unlink'}<p class="case-multiline">
        Motivo: {prepared.command.change.reason}
      </p>{/if}
    <p>Autor: {prepared.recorded_by.email}</p>
    <div class="action-row">
      <button class="primary" disabled={frozen} onclick={submit}
        >{action === 'link' ? 'Confirmar v\u00ednculo' : 'Confirmar desvinculaci\u00f3n'}</button
      >
      <button
        class="secondary"
        disabled={frozen}
        onclick={() => {
          mode = 'draft';
          prepared = null;
        }}>Volver al borrador</button
      >
    </div>
  {/if}
  {#if blocked}
    {#if action === 'link'}<label
        >Tipo de actividad<select disabled
          ><option value="">Selecciona el tipo de actividad</option></select
        ></label
      >
    {:else}<label>Motivo<textarea disabled /></label>{/if}
    <button class="primary" disabled
      >{action === 'link' ? 'Preparar v\u00ednculo' : 'Preparar desvinculaci\u00f3n'}</button
    >
  {:else if mode !== 'review' && mode !== 'confirmed'}
    {#if action === 'link'}
      <ResourceActivityFields
        bind:this={fields}
        savedInputs={inputs}
        {api}
        {caseId}
        {resource}
        bind:selection
        {ondenied}
        disabled={frozen || mode !== 'draft'}
        bind:busy={fieldsBusy}
      />
    {:else}
      <label
        >Motivo<textarea
          bind:value={reason}
          maxlength="2000"
          disabled={frozen || mode !== 'draft'}
        /></label
      >
      <p>La captura vinculada permanecer&#225; en la historia.</p>
    {/if}
    <button
      class="primary"
      disabled={frozen ||
        mode !== 'draft' ||
        (action === 'link' ? !selection.target : !reason.trim())}
      onclick={prepare}
      >{action === 'link' ? 'Preparar v\u00ednculo' : 'Preparar desvinculaci\u00f3n'}</button
    >
  {/if}
  <button class="text-button" disabled={pending || disabled || blocked} onclick={close}
    >Cerrar formulario</button
  >
</section>
