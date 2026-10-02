<script>
  import FactPersonFields from './FactPersonFields.svelte';
  import FactProvenanceFields from './FactProvenanceFields.svelte';
  export let value,
    api,
    caseId,
    ondenied,
    disabled = false,
    pending = false,
    inputs = null,
    supportContext = () => null,
    discardSupport = () => {},
    supportDenied = ondenied,
    canApply = () => true;
  let representedBusy = false,
    representativeBusy = false,
    provenanceBusy = false,
    represented,
    representative,
    provenance,
    epoch = 0,
    savedInputs = inputs;
  $: pending = representedBusy || representativeBusy || provenanceBusy;
  const supportPath = ['representation', 'provenance', 'support'];
  export function captureDraft() {
    return {
      represented: represented?.captureDraft() ?? savedInputs?.represented ?? null,
      representative: representative?.captureDraft() ?? savedInputs?.representative ?? null,
      provenance: provenance?.captureDraft() ?? savedInputs?.provenance ?? null,
    };
  }
  function kind(next) {
    if (!canApply()) return;
    discardSupport(supportPath);
    epoch++;
    savedInputs = null;
    value =
      next === 'not_recorded'
        ? { kind: next, reason: '' }
        : next === 'declared'
          ? {
              kind: next,
              represented: { kind: '' },
              representative: { kind: '' },
              scope: '',
              provenance: { kind: '' },
            }
          : { kind: '' };
  }
</script>

<fieldset class="case-offenses fact-representation-fields" {disabled}>
  <legend>Representaci&#243;n declarada</legend>
  <label
    >Representaci&#243;n declarada<select
      value={value?.kind || ''}
      onchange={(event) => kind(event.currentTarget.value)}
      disabled={pending}
    >
      <option value="">Selecciona lo declarado</option><option value="not_recorded"
        >No registrada</option
      ><option value="declared">V&#237;nculo declarado expresamente</option>
    </select></label
  >
  {#if value?.kind === 'not_recorded'}<label
      >Motivo: Representaci&#243;n declarada<textarea
        rows="2"
        bind:value={value.reason}
        disabled={pending}></textarea></label
    >
  {:else if value?.kind === 'declared'}{#key epoch}
      <FactPersonFields
        bind:value={value.represented}
        label="Persona representada"
        draft={savedInputs?.represented ?? null}
        {canApply}
        bind:this={represented}
        {api}
        {caseId}
        {ondenied}
        declared={false}
        disabled={disabled || representativeBusy || provenanceBusy}
        bind:pending={representedBusy}
      />
      <FactPersonFields
        bind:value={value.representative}
        label="Persona representante"
        draft={savedInputs?.representative ?? null}
        {canApply}
        bind:this={representative}
        {api}
        {caseId}
        {ondenied}
        declared={false}
        disabled={disabled || representedBusy || provenanceBusy}
        bind:pending={representativeBusy}
      />
      <label
        >Alcance de la representaci&#243;n<textarea
          rows="3"
          bind:value={value.scope}
          disabled={pending}></textarea></label
      >
      <FactProvenanceFields
        bind:value={value.provenance}
        label="Procedencia de la representaci&#243;n"
        draft={savedInputs?.provenance ?? null}
        {canApply}
        bind:this={provenance}
        draftContext={supportContext(supportPath)}
        ondiscard={() => discardSupport(supportPath)}
        onsupportdenied={(failure) => supportDenied(failure, supportPath)}
        {api}
        {caseId}
        {ondenied}
        disabled={disabled || representedBusy || representativeBusy}
        bind:pending={provenanceBusy}
      />
      <p class="hint">
        El v&#237;nculo es una declaraci&#243;n expresa. No se deduce del rol de las fichas ni
        acredita facultades legales.
      </p>
    {/key}{/if}
</fieldset>
