<script>
  import FactDeclarationFields from './FactDeclarationFields.svelte';
  import FactTimeFields from './FactTimeFields.svelte';
  import FactPersonFields from './FactPersonFields.svelte';
  import FactRepresentationFields from './FactRepresentationFields.svelte';
  import FactProvenanceFields from './FactProvenanceFields.svelte';
  import FactResolutionPicker from './FactResolutionPicker.svelte';
  import {
    classLabels,
    characterLabels,
    mediumLabels,
    contextLabels,
    outcomeLabels,
  } from './fact-field-labels.mjs';
  export let values,
    family,
    api,
    caseId,
    resolution = null,
    ondenied,
    disabled = false,
    pending = false;
  let recipientBusy = false,
    receiverBusy = false,
    representationBusy = false,
    provenanceBusy = false,
    parentBusy = false,
    choosingParent = false;
  $: pending = recipientBusy || receiverBusy || representationBusy || provenanceBusy || parentBusy;
  $: locked = disabled || pending;
</script>

<div class="hearing-fields fact-fields">
  {#if family === 'resolution'}
    <FactDeclarationFields
      bind:value={values.class}
      label="Clase de resoluci&#243;n"
      choices={classLabels}
      disabled={locked}
    />
    <FactDeclarationFields bind:value={values.issuer} label="Emisor" disabled={locked} />
    <FactTimeFields bind:value={values.issued_at} label="emisi&#243;n" disabled={locked} />
  {:else}
    <section class="case-comparison" aria-label="Resoluci&#243;n vinculada">
      <h4>Resoluci&#243;n vinculada</h4>
      <p>Revisi&#243;n seleccionada: {values.resolution.revision}</p>
      <details>
        <summary>Identidad fija de la resoluci&#243;n</summary><code>{values.resolution.id}</code>
      </details>
      <button
        type="button"
        class="secondary"
        disabled={locked}
        onclick={() => (choosingParent = true)}>Elegir revisi&#243;n de la resoluci&#243;n</button
      >
      {#if choosingParent}<FactResolutionPicker
          {api}
          {caseId}
          resolutionId={resolution?.id || values.resolution.id}
          {ondenied}
          disabled={disabled ||
            recipientBusy ||
            receiverBusy ||
            representationBusy ||
            provenanceBusy}
          bind:busy={parentBusy}
          onselected={(row) => {
            values = { ...values, resolution: { id: row.id, revision: row.revision } };
            choosingParent = false;
          }}
          oncancel={() => (choosingParent = false)}
        />{/if}
    </section>
    <FactDeclarationFields
      bind:value={values.character}
      label="Car&#225;cter de notificaci&#243;n"
      choices={characterLabels}
      disabled={locked}
    />
    <FactDeclarationFields
      bind:value={values.medium}
      label="Medio de notificaci&#243;n"
      choices={mediumLabels}
      disabled={locked}
    />
    <FactDeclarationFields
      bind:value={values.context}
      label="Contexto de notificaci&#243;n"
      choices={contextLabels}
      disabled={locked}
    />
    <FactDeclarationFields
      bind:value={values.outcome}
      label="Resultado declarado"
      choices={outcomeLabels}
      disabled={locked}
    />
    <FactTimeFields bind:value={values.practiced_at} label="pr&#225;ctica" disabled={locked} />
    <label class="checkbox"
      ><input
        type="checkbox"
        checked={values.received_at !== null}
        disabled={locked}
        onchange={(event) =>
          (values = {
            ...values,
            received_at: event.currentTarget.checked ? { precision: '' } : null,
          })}
      />Registrar tiempo de recepci&#243;n</label
    >
    {#if values.received_at !== null}<FactTimeFields
        bind:value={values.received_at}
        label="recepci&#243;n"
        disabled={locked}
      />{/if}
    <label class="checkbox"
      ><input
        type="checkbox"
        checked={values.stated_effect !== null}
        disabled={locked}
        onchange={(event) =>
          (values = {
            ...values,
            stated_effect: event.currentTarget.checked
              ? { at: { precision: '' }, statement: '', locator: '' }
              : null,
          })}
      />Registrar efecto expresamente declarado</label
    >
    {#if values.stated_effect}<fieldset class="case-offenses" disabled={locked}>
        <legend>Efecto expresamente declarado</legend>
        <FactTimeFields
          bind:value={values.stated_effect.at}
          label="efecto declarado"
          disabled={locked}
        />
        <label
          >Declaraci&#243;n del efecto<textarea rows="3" bind:value={values.stated_effect.statement}
          ></textarea></label
        >
        <label
          >Localizador del efecto en la procedencia principal<input
            bind:value={values.stated_effect.locator}
          /></label
        >
        <p class="hint">
          Se registra una afirmaci&#243;n de la fuente; no se calcula ni se acredita su eficacia.
        </p>
      </fieldset>{/if}
    <FactPersonFields
      bind:value={values.intended_recipient}
      label="Destinatario declarado"
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || receiverBusy || representationBusy || provenanceBusy || parentBusy}
      bind:pending={recipientBusy}
    />
    <FactPersonFields
      bind:value={values.actual_receiver}
      label="Receptor material"
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || recipientBusy || representationBusy || provenanceBusy || parentBusy}
      bind:pending={receiverBusy}
    />
    <FactRepresentationFields
      bind:value={values.representation}
      {api}
      {caseId}
      {ondenied}
      disabled={disabled || recipientBusy || receiverBusy || provenanceBusy || parentBusy}
      bind:pending={representationBusy}
    />
  {/if}
  <label
    >Subtipo declarado (opcional)<input
      value={values.subtype || ''}
      disabled={locked}
      oninput={(event) => (values = { ...values, subtype: event.currentTarget.value || null })}
    /></label
  >
  <label
    >{family === 'resolution'
      ? 'Resumen de la resoluci\u00f3n'
      : 'Resumen de la notificaci\u00f3n'}<textarea
      rows="4"
      bind:value={values.summary}
      disabled={locked}></textarea></label
  >
  <FactProvenanceFields
    bind:value={values.provenance}
    label={family === 'resolution'
      ? 'Procedencia de la resoluci\u00f3n'
      : 'Procedencia de la notificaci\u00f3n'}
    {api}
    {caseId}
    {ondenied}
    disabled={disabled || recipientBusy || receiverBusy || representationBusy || parentBusy}
    bind:pending={provenanceBusy}
  />
</div>
