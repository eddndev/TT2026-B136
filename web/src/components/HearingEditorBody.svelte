<script>
  import HearingFields from './HearingFields.svelte';
  import HearingValues from './HearingValues.svelte';
  import HearingReconciliation from './HearingReconciliation.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let action,
    draft,
    rows,
    base,
    context,
    participantsApi,
    typedApi,
    documents,
    caseId,
    mode,
    prepared,
    candidate,
    comparedContext,
    comparisonReady,
    canAccept,
    pending,
    frozen,
    error,
    issue,
    fieldsDisabled,
    fieldsBusy,
    blocked,
    restoredClosed,
    selectors,
    supportContext,
    supportDenied,
    ondenied,
    canApply,
    prepare,
    submit,
    check,
    refresh,
    accept,
    close,
    retry,
    back,
    reviewed,
    discardSupport;
  export let last, busy;
  let fields;
  export function captureSelectors() {
    return fields?.captureDraft() ?? selectors;
  }
</script>

<section
  class="card case-editor hearing-editor"
  aria-label="Formulario de audiencia"
  aria-busy={pending}
>
  <h2>
    {action === 'schedule'
      ? 'Programar audiencia'
      : action === 'replace'
        ? 'Corregir o reprogramar audiencia'
        : 'Cancelar audiencia'}
  </h2>
  <p class="hint">
    La programaci&#243;n conserva lo declarado. No registra celebraci&#243;n, asistencia ni
    resultados.
  </p>
  <CaseClosedNotice />
  {#if blocked || restoredClosed}<button class="secondary" disabled={pending} onclick={retry}
      >Volver a consultar el contexto</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if issue}<p class="notice">
      {issue === 'participants'
        ? 'Quita o vuelve a seleccionar las fichas que requieren revisi\u00f3n antes de preparar.'
        : 'Elige y consulta de nuevo el soporte exacto antes de preparar.'}
    </p>{/if}
  {#if ['conflict', 'uncertain'].includes(mode)}<HearingReconciliation
      {mode}
      {last}
      {candidate}
      context={comparedContext}
      ready={comparisonReady}
      {canAccept}
      busy={pending}
      oncheck={check}
      onrefresh={refresh}
      onaccept={accept}
    />{/if}
  {#if prepared}<div class="case-comparison hearing-preview">
      <h3>Confirma el registro</h3>
      <p>
        Revisi&#243;n esperada {prepared.command.change.expected_revision} / Revisi&#243;n a registrar
        {prepared.result_revision}.
      </p>
      <HearingValues values={prepared.values} participants={rows} support={draft.support} />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <div class="action-row">
        <button class="secondary" disabled={pending} onclick={back}>Volver al borrador</button>
        <button class="primary" disabled={frozen} onclick={submit}
          >{busy ? 'Registrando audiencia...' : 'Confirmar registro'}</button
        >
      </div>
    </div>
  {:else if mode !== 'confirmed'}
    {#if action === 'cancel'}<HearingValues
        values={base.values}
        participants={rows}
        support={base.support}
      />
      <label
        >Motivo de cancelaci&#243;n<textarea rows="3" bind:value={draft.reason} disabled={frozen}
        ></textarea></label
      >
      <p class="hint">
        Se conserva la programaci&#243;n y su historia. La cancelaci&#243;n organizativa no declara
        nulidad procesal.
      </p>
    {:else}{#key blocked}<HearingFields
          bind:draft
          bind:rows
          {base}
          {context}
          api={participantsApi}
          {typedApi}
          {documents}
          {caseId}
          {ondenied}
          {supportContext}
          onsupportdenied={supportDenied}
          {discardSupport}
          {canApply}
          {selectors}
          bind:this={fields}
          disabled={fieldsDisabled}
          bind:pending={fieldsBusy}
          onreviewed={reviewed}
        />{/key}{/if}
    <button class="primary" disabled={frozen || mode !== 'draft' || !!issue} onclick={prepare}
      >Revisar registro</button
    >
  {/if}
  {#if mode === 'uncertain'}<p class="hint">
      Cerrar el formulario descarta su borrador local; no cancela un env&#237;o que pueda estar en
      proceso.
    </p>{/if}
  <button class="text-button" disabled={pending} onclick={close}
    >Cerrar formulario de audiencia</button
  >
</section>
