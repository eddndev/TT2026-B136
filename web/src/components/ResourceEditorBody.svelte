<script>
  import ResourceFields from './ResourceFields.svelte';
  import ResourceActFields from './ResourceActFields.svelte';
  import ResourceValues from './ResourceValues.svelte';
  import ResourceSources from './ResourceSources.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  import { resourceDraft, resourceActions } from '../lib/procedural-resource-values.mjs';
  export let api,
    caseId,
    user,
    action,
    draft,
    mode,
    prepared,
    candidate,
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
  $: isAct = ['record_act', 'correct_act'].includes(action);
  let fields;
  export function captureInputs() {
    return fields?.captureDraft() ?? inputs;
  }
</script>

<section
  class="card case-editor fact-editor"
  aria-label="Formulario de recurso"
  aria-busy={pending}
>
  <h2>{resourceActions[action]}</h2>
  <p class="hint">
    Conserva lo declarado y sus fuentes. El registro no determina efectos jur&#237;dicos ni inicia
    plazos.
  </p>
  <CaseClosedNotice />
  {#if blocked || restoredClosed}<button class="secondary" disabled={pending} onclick={retry}
      >Volver a consultar el contexto</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if !blocked && mode === 'uncertain'}
    <div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. Una ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={pending} onclick={check}>Consultar envio exacto</button>
    </div>
  {:else if !blocked && mode === 'conflict'}
    <div class="case-comparison">
      <h3>El registro cambi&#243;</h3>
      <p>Tu borrador se conserva. Consulta la base actual antes de decidir.</p>
      <button class="secondary" disabled={pending} onclick={compare}
        >Comparar con registro actual</button
      >
      {#if candidate}<p>
          Revisi&#243;n {candidate.revision} / {candidate.status === 'active'
            ? 'Activo'
            : 'Archivado'}
        </p>
        <ResourceValues values={candidate.values} />
        {#if action === 'correct_act'}<p>
            Consulta en la historia la &#250;ltima revisi&#243;n del acto y abre su correcci&#243;n.
            Este borrador permanece visible hasta cerrar el formulario.
          </p>
        {:else}<button
            class="primary"
            disabled={frozen ||
              action === 'register' ||
              (action === 'reactivate') !== (candidate.status === 'archived')}
            onclick={accept}>Usar base actual y conservar borrador</button
          >{/if}
      {/if}
    </div>
  {:else if !blocked && mode === 'review' && prepared}
    <h3>Revisar antes de confirmar</h3>
    <ResourceValues values={prepared.values} />
    {#if prepared.act}<h3>Acto declarado</h3>
      <ResourceValues values={prepared.act.values} act />{/if}
    <ResourceSources sources={prepared.sources} act={prepared.act} />
    {#if draft.reason}<p class="case-multiline">Motivo: {draft.reason}</p>{/if}
    <div class="action-row">
      <button class="primary" disabled={frozen} onclick={submit}>Confirmar registro</button>
      <button class="secondary" disabled={pending} onclick={back}>Volver al borrador</button>
    </div>
  {/if}
  {#if (mode !== 'review' && mode !== 'confirmed') || blocked}
    <fieldset disabled={frozen || mode !== 'draft'}>
      {#if blocked}
        {#if isAct}<ResourceActFields
            {api}
            {caseId}
            {user}
            values={resourceDraft(null, true).values}
            {ondenied}
            disabled
          />
        {:else if ['register', 'correct'].includes(action)}<ResourceFields
            {api}
            {caseId}
            {user}
            values={resourceDraft(null, false).values}
            {ondenied}
            disabled
          />{/if}
      {:else if isAct}<ResourceActFields
          {api}
          {caseId}
          {user}
          bind:values={draft.values}
          {ondenied}
          disabled={fieldsDisabled || mode !== 'draft'}
          bind:busy={fieldsBusy}
          {inputs}
          {supportContext}
          {discardSupport}
          {supportDenied}
          {canApply}
          bind:this={fields}
        />
      {:else if ['register', 'correct'].includes(action)}<ResourceFields
          {api}
          {caseId}
          {user}
          bind:values={draft.values}
          {ondenied}
          disabled={fieldsDisabled || mode !== 'draft'}
          bind:busy={fieldsBusy}
          {inputs}
          {supportContext}
          {discardSupport}
          {supportDenied}
          {canApply}
          bind:this={fields}
        />
      {:else}<p>El archivo es organizativo. No declara desistimiento ni modifica actos previos.</p>
        <ResourceValues values={draft.values} />{/if}
      {#if ['correct', 'correct_act', 'archive', 'reactivate'].includes(action)}
        {#if blocked}<label>Motivo<textarea value="" disabled rows="3"></textarea></label>
        {:else}<label
            >Motivo<textarea bind:value={draft.reason} maxlength="1000" rows="3"></textarea></label
          >{/if}
      {/if}
    </fieldset>
    {#if mode === 'draft' || blocked}<button class="primary" disabled={frozen} onclick={prepare}
        >Preparar registro</button
      >{/if}
  {/if}
  <button class="text-button" disabled={pending} onclick={close}>Cerrar formulario</button>
</section>
