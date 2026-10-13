<script>
  import HearingResultSources from './HearingResultSources.svelte';
  import HearingResultFields from './HearingResultFields.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import HearingAnchorPicker from './HearingAnchorPicker.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let action,
    draft,
    rows,
    base,
    anchor,
    source,
    candidates,
    caseId,
    participants,
    typed,
    documents,
    hearings,
    mode,
    prepared,
    last,
    candidate,
    compared,
    error,
    pending,
    frozen,
    fieldsDisabled,
    fieldsBusy,
    pickerBusy,
    picking,
    blocked,
    restoredClosed,
    selectors,
    supportContext,
    supportDenied,
    ondenied,
    canApply,
    discardSupport,
    prepare,
    submit,
    check,
    compare,
    accept,
    close,
    retry,
    back,
    selectAnchor;
  let fields;
  export function captureSelectors() {
    return fields?.captureDraft() ?? selectors;
  }
</script>

<section
  class="card case-editor hearing-result-editor"
  aria-label="Formulario de sesi&#243;n o acto"
  aria-busy={pending}
>
  <h2>
    {action === 'record'
      ? source
        ? 'Registrar continuaci\u00f3n'
        : 'Registrar sesi\u00f3n o acto'
      : action === 'correct'
        ? 'Rectificar registro'
        : 'Retirar registro'}
  </h2>
  <p class="hint">
    Conserva lo comunicado con sus fuentes. Esta captura no declara notificaci&#243;n, firmeza ni
    plazos.
  </p>
  <CaseClosedNotice />
  {#if blocked || restoredClosed}<button class="secondary" disabled={pending} onclick={retry}
      >Volver a consultar el contexto</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  <HearingResultSources {anchor} continuation={source} />
  {#if action === 'record' && mode === 'draft'}<button
      class="secondary"
      disabled={frozen}
      onclick={() => (picking = true)}>Elegir programaci&#243;n de origen</button
    >{/if}
  {#if picking}<HearingAnchorPicker
      api={hearings}
      {ondenied}
      {canApply}
      disabled={fieldsDisabled || fieldsBusy}
      bind:busy={pickerBusy}
      onselected={selectAnchor}
      oncancel={() => (picking = false)}
    />{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending} onclick={check}>Consultar env&#237;o exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con el registro actual</h3>
      <button class="secondary" disabled={pending} onclick={compare}
        >Consultar base actual del resultado</button
      >
      {#if compared && candidate}<p>
          Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'withdrawn'
            ? 'Registro retirado'
            : 'Registrado'}
        </p>
        <HearingResultValues
          values={candidate.values}
          attendees={candidate.attendees}
          support={candidate.support}
        />
        {#if action !== 'record' && candidate.status === 'recorded'}<button
            class="primary"
            disabled={frozen}
            onclick={accept}>Usar esta base y conservar borrador</button
          >{:else}<p>
            Esta captura no permite reenviar el borrador sobre la base consultada. Puedes cerrar y
            revisar sus fuentes.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el resultado a registrar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <HearingResultValues
        values={prepared.values}
        attendees={prepared.attendees}
        support={prepared.support}
      />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <div class="action-row">
        <button class="secondary" disabled={pending} onclick={back}
          >Volver al borrador del resultado</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar resultado</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'withdraw'}<HearingResultValues
        values={base.values}
        attendees={rows}
        support={base.support}
      />
      <p class="notice">Retirar conserva la captura y su historia; no anula el acto informado.</p>
    {:else}{#key blocked}<HearingResultFields
          bind:draft
          bind:rows
          {candidates}
          {caseId}
          {participants}
          {typed}
          {documents}
          {ondenied}
          {canApply}
          {supportContext}
          onsupportdenied={supportDenied}
          {discardSupport}
          {selectors}
          bind:this={fields}
          disabled={fieldsDisabled || pickerBusy}
          bind:pending={fieldsBusy}
        />{/key}{/if}
    {#if action !== 'record'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft' || picking} onclick={prepare}
      >Revisar resultado</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={close}
    >Cerrar formulario de resultado</button
  >
</section>
