<script>
  import StageFields from './StageFields.svelte';
  import StageValues from './StageValues.svelte';
  import StageEntry from './StageEntry.svelte';
  import { stageAction, stageLabels } from '../lib/case-stages.mjs';
  import { stageReferences } from '../lib/stage-form-actions.mjs';
  export let state, actions, documents, caseId, ondenied, close, retry;
  export let disabled, readBlocked, supportUnreviewed, supportContext, discardSupport;
  export let pending = false;
  let fieldsBusy = false;
  $: pending = state.busy || fieldsBusy;
</script>

<section class="card case-editor stage-form" aria-busy={pending}>
  <h2>
    {state.action === 'adoption'
      ? 'Registrar etapa actual'
      : `Registrar paso a ${stageLabels[state.action]}`}
  </h2>
  <p class="hint">
    Se registran los datos que declaras y sus soportes. El sistema no certifica la procedencia
    jur&#237;dica del cambio.
  </p>
  {#if state.error}<p class="notice error" role="alert">{state.error}</p>{/if}
  {#if state.blocked || state.restoredClosed || state.incomplete}<button
      class="secondary"
      disabled={pending || readBlocked}
      onclick={retry}>Volver a consultar la etapa y sus soportes</button
    >{/if}
  {#if state.restoredClosed || state.incomplete}<p class="notice" role="status">
      Este borrador se conserva para lectura. El expediente requiere estado activo y ficha penal
      completa para registrar una etapa.
    </p>{/if}
  {#if supportUnreviewed}<p class="notice">
      Vuelve a elegir y consultar cada versi&#243;n seleccionada. Se conserva el borrador y no se
      cargar&#225;n archivos de nuevo autom&#225;ticamente.
    </p>{/if}
  {#if state.needsReview}
    <button
      class="secondary"
      disabled={pending || readBlocked || state.blocked}
      onclick={actions.reconcile}>Consultar etapa e historial</button
    >
    {#if state.lastPayload}<details class="stage-last-request">
        <summary>Consultar el &#250;ltimo env&#237;o</summary>
        <p>Revisi&#243;n esperada: {state.lastPayload.expected_revision}.</p>
        <StageValues values={state.lastPayload} supports={state.lastSupports} />
      </details>{/if}
    {#if state.candidate !== undefined}<div class="case-comparison stage-reconciliation">
        <h3>Etapa consultada</h3>
        {#if state.candidate}<StageEntry record={state.candidate} />{:else}<p>
            Sin etapa registrada.
          </p>{/if}
        {#if state.uncertain}<p class="notice">
            Una coincidencia no confirma que este env&#237;o se guard&#243;. Revisa los registros
            antes de decidir otro intento.
          </p>{/if}
        {#if stageAction(state.candidate) === state.action}<button
            class="secondary"
            disabled={pending || disabled || !state.historyComplete}
            onclick={actions.accept}>Usar etapa consultada y revisar borrador</button
          >
        {:else}<p class="notice">
            El avance del borrador ya no corresponde a la etapa consultada. Conserva estos datos
            antes de cerrar el formulario.
          </p>{/if}
        <details>
          <summary>Historial consultado para comparar</summary>
          {#each state.compared as record (record.stage_revision)}<StageEntry {record} />{/each}
        </details>
      </div>{/if}
  {/if}
  {#if state.preview}<div class="case-comparison stage-confirmation">
      <h3>Confirma el registro</h3>
      <p>
        {state.action === 'adoption'
          ? 'Adopci\u00f3n de etapa conocida'
          : `${stageLabels[state.base.stage]} a ${stageLabels[state.action]}`} / revisi&#243;n esperada
        {state.preview.expected_revision}.
      </p>
      <StageValues values={state.preview} supports={stageReferences(state.draft)} />
      <p>El registro quedar&#225; en el historial. Revisa los datos antes de confirmarlo.</p>
      <div class="action-row">
        <button
          class="secondary"
          disabled={pending || disabled}
          onclick={() => (state.preview = null)}>Volver al borrador</button
        >
        <button class="primary" disabled={pending || disabled} onclick={actions.submit}
          >{state.busy
            ? 'Registrando etapa...'
            : state.action === 'adoption'
              ? 'Registrar etapa actual'
              : 'Registrar transici\u00f3n'}</button
        >
      </div>
    </div>
  {:else}
    {#key state.blocked}<StageFields
        action={state.action}
        bind:draft={state.draft}
        api={documents}
        {caseId}
        {ondenied}
        {supportContext}
        {discardSupport}
        disabled={disabled || state.busy}
        bind:pending={fieldsBusy}
      />{/key}
    <div class="action-row">
      <button
        class="primary"
        disabled={disabled || pending || state.needsReview || state.exhausted || supportUnreviewed}
        onclick={actions.review}>Revisar registro</button
      >
    </div>
  {/if}
  <button class="text-button" disabled={pending} onclick={close}>Cerrar formulario de etapa</button>
</section>
