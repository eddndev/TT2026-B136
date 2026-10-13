<script>
  import FactFields from './FactFields.svelte';
  import FactValues from './FactValues.svelte';
  import FactSources from './FactSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let api,
    caseId,
    family,
    resolution,
    action,
    base,
    draft,
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
    blocked,
    restoredClosed,
    inputs,
    supportContext,
    discardSupport,
    supportDenied,
    canApply,
    ondenied,
    prepare,
    submit,
    check,
    compare,
    accept,
    close,
    retry,
    back;
  const singular = family === 'resolution' ? 'resoluci\u00f3n' : 'notificaci\u00f3n';
  let fields;
  export function captureInputs() {
    return fields?.captureDraft() ?? inputs;
  }
</script>

<section
  class="card case-editor fact-editor"
  aria-label={`Formulario de ${singular}`}
  aria-busy={pending}
>
  <h2>
    {action === 'record' ? 'Registrar' : action === 'correct' ? 'Corregir' : 'Retirar'}
    {singular}
  </h2>
  <p class="hint">
    Registra lo declarado con sus fuentes. La captura no determina efectos jur&#237;dicos ni inicia
    plazos.
  </p>
  <CaseClosedNotice />
  {#if blocked || restoredClosed}<button class="secondary" disabled={pending} onclick={retry}
      >Volver a consultar el contexto</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending || blocked} onclick={check}
        >Consultar env&#237;o exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con el registro actual</h3>
      <button class="secondary" disabled={pending || blocked} onclick={compare}
        >Consultar base actual</button
      >
      {#if compared && candidate}<p>
          Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'withdrawn'
            ? 'Retirado'
            : 'Registrado'}
        </p>
        <FactValues values={candidate.values} {family} /><FactSources sources={candidate.sources} />
        {#if action !== 'record' && candidate.status === 'recorded'}<button
            class="primary"
            disabled={frozen}
            onclick={accept}>Usar esta base y conservar borrador</button
          >
        {:else}<p>
            Esta captura no permite reenviar sobre la base consultada. Puedes cerrar y revisar sus
            fuentes.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el registro a confirmar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <FactValues values={prepared.values} {family} /><FactSources
        sources={prepared.sources}
      /><FactAdministrativeCapture value={prepared.observed_administration} />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <div class="action-row">
        <button class="secondary" disabled={pending} onclick={back}>Volver al borrador</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar registro</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'withdraw'}<FactValues values={base.values} {family} /><FactSources
        sources={base.sources}
      />
      <p class="notice">Retirar conserva la captura y su historia; no anula el acto declarado.</p>
    {:else}{#key blocked}<FactFields
          bind:values={draft.values}
          {family}
          {api}
          {caseId}
          {resolution}
          {ondenied}
          disabled={fieldsDisabled}
          {inputs}
          {supportContext}
          {discardSupport}
          {supportDenied}
          {canApply}
          recoverable={true}
          bind:this={fields}
          bind:pending={fieldsBusy}
        />{/key}{/if}
    {#if action !== 'record'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={prepare}
      >Preparar registro</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={close}>Cerrar formulario</button>
</section>
