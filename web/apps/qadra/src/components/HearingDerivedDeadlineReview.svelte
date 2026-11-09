<script>
  import HearingResultSources from './HearingResultSources.svelte';
  import HearingResultValues from './HearingResultValues.svelte';
  import DeadlineCalculation from './DeadlineCalculation.svelte';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { roles } from '../lib/documents.mjs';
  import {
    deadlinePolicyLabels,
    deadlinePurposes,
    deadlineDeclarationLabel,
  } from './deadline-view-labels.mjs';
  export let prepared;
  $: result = prepared.result;
  $: deadline = prepared.deadline;
  $: input = deadline.definition.input;
  $: selection = input.selection;
  $: agreementId = selection.source.value.agreement_id;
</script>

<div class="case-comparison" aria-label="Revisi&#243;n conjunta">
  <h3>Revisa el resultado y el plazo</h3>
  <p>Ambos registros se guardan juntos como revisi&#243;n 1.</p>
  <HearingResultSources anchor={result.anchor} continuation={result.continuation} />
  <HearingResultValues
    values={result.values}
    attendees={result.attendees}
    support={result.support}
  />
  <h4>{deadline.definition.title}</h4>
  <p>
    Responsable: {deadline.responsible.email} / {roles[deadline.responsible.role] ??
      deadline.responsible.role}
  </p>
  <p>
    Perfil seleccionado: revisi&#243;n {deadline.profile.revision}; cabeza observada: {deadline
      .profile_head.revision}.
  </p>
  <p>
    Seguimiento del perfil: {deadlinePolicyLabels[deadline.tracking.profile]}; de la fuente: {deadlinePolicyLabels[
      deadline.tracking.source
    ]}; del calendario: {deadlinePolicyLabels[deadline.tracking.calendar]}.
  </p>
  {#if deadline.calendar}
    <p>
      Calendario seleccionado: revisi&#243;n {deadline.calendar.revision}; cabeza observada: {deadline
        .calendar_head.revision}.
    </p>
  {:else}<p>Sin calendario seleccionado.</p>{/if}
  {#if agreementId}
    <p class="case-multiline">
      Acuerdo seleccionado: {result.values.agreements.find((row) => row.id === agreementId).text}
    </p>
    <details>
      <summary>Identidad del acuerdo seleccionado</summary><code>{agreementId}</code>
    </details>
  {:else}<p>Resultado completo, sin acuerdo espec&#237;fico.</p>{/if}
  {#if selection.qualification}
    <p>
      {deadlinePurposes[selection.qualification.purpose]}: {factTimeLabel(
        selection.qualification.at,
      )}
    </p>
    <p class="case-multiline">
      {selection.qualification.statement} / {selection.qualification.locator}
    </p>
  {:else}<p>Sin inicio calificado declarado.</p>{/if}
  <p>Cantidad ordenada: {input.ordered_quantity ?? 'Ausente'}</p>
  <p class="case-multiline">
    Aplicabilidad declarada: {deadline.definition.input.qualification.statement}
  </p>
  <p>Localizador: {deadline.definition.input.qualification.locator}</p>
  <p>El &#225;mbito aplica: {deadlineDeclarationLabel(input.qualification.scope_applies)}</p>
  <p>
    Existe incidencia sin resolver: {deadlineDeclarationLabel(
      input.qualification.unresolved_incident,
    )}
  </p>
  {#each input.qualification.conditions as condition, index}
    <p class="case-multiline">
      Condici&#243;n {index + 1}: {deadline.profile.definition.conditions.find(
        (row) => row.id === condition.id,
      )?.statement || condition.id} / {deadlineDeclarationLabel(condition.applies)} / {condition.locator}
    </p>
  {/each}
  <DeadlineCalculation
    prospective
    calculation={{
      result: deadline.result,
      profile: { title: deadline.profile.definition.title, revision: deadline.profile.revision },
    }}
  />
  <details>
    <summary>Huella de la revisi&#243;n conjunta</summary><code>{prepared.review_digest}</code>
  </details>
</div>
