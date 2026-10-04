<script>
  import HearingResultFields from './HearingResultFields.svelte';
  import DeadlineFields from './DeadlineFields.svelte';
  export let draft,
    rows,
    candidates,
    caseId,
    api,
    participants,
    typed,
    documents,
    definition,
    policies,
    ondenied,
    supportDenied,
    supportContext,
    discardSupport,
    canApply,
    disabled = false,
    pending = false,
    inputs = null;
  let resultFields,
    deadlineFields,
    resultBusy = false,
    deadlineBusy = false;
  export function captureInputs() {
    return {
      result: resultFields?.captureDraft() ?? inputs?.result ?? null,
      deadline: deadlineFields?.captureInputs() ?? inputs?.deadline ?? null,
    };
  }
  $: pending = resultBusy || deadlineBusy;
</script>

<h3>Resultado a registrar</h3>
<HearingResultFields
  bind:this={resultFields}
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
  {discardSupport}
  onsupportdenied={supportDenied}
  selectors={inputs?.result ?? null}
  disabled={disabled || deadlineBusy}
  bind:pending={resultBusy}
/>
<h3>Plazo configurado</h3>
<p class="hint">
  Selecciona la regla y declara su aplicabilidad. El relato no determina consecuencias juridicas por
  si solo.
</p>
<DeadlineFields
  bind:this={deadlineFields}
  bind:value={definition}
  bind:policies
  {api}
  {caseId}
  {ondenied}
  prospective={{ agreements: draft.agreements }}
  recoverable
  savedInputs={inputs?.deadline ?? null}
  disabled={disabled || resultBusy}
  bind:pending={deadlineBusy}
/>
