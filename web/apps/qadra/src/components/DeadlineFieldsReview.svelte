<script>
  import DeadlineCalculation from './DeadlineCalculation.svelte';
  import DeadlineTrackingSummary from './DeadlineTrackingSummary.svelte';
  import DeadlineReceipt from './DeadlineReceipt.svelte';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  export let value,
    profile = null;
  $: definition = value.definition;
  $: selection = definition.input.selection;
  const declaration = (v) =>
    v.kind === 'unknown' ? `No consta: ${v.reason}` : v.value ? 'Si' : 'No';
  const families = {
    resolution: 'Resolucion',
    notification: 'Notificacion',
    hearing_result: 'Resultado de audiencia',
  };
</script>

<div class="calendar-values">
  <h4>{definition.title}</h4>
  <p>Responsable: {value.responsible.email} / {value.responsible.role}</p>
  <DeadlineTrackingSummary {value} prepared={!!value.command} historical={!value.command} />
  <DeadlineReceipt {value} compact />
  <p>Perfil: {value.calculation.profile.title} / Revisi&#243;n {definition.profile.revision}</p>
  {#if selection.source.kind === 'unknown'}<p>Fuente no identificada: {selection.source.reason}</p>
  {:else}<p>
      {families[selection.source.value.family]} / Revisi&#243;n {selection.source.value.revision}
    </p>
    {#if selection.source.value.resolution}<p>
        Resoluci&#243;n vinculada revisi&#243;n {selection.source.value.resolution.revision}
      </p>{/if}
    {#if selection.source.value.family === 'hearing_result'}<p>
        {selection.source.value.agreement_id === null
          ? 'Resultado completo, sin acuerdo especifico'
          : `Acuerdo: ${selection.source.value.agreement_id}`}
      </p>{/if}
    <details>
      <summary>Identidad de la fuente seleccionada</summary><code
        >{selection.source.value.id || selection.source.value.result_id}</code
      >
    </details>
  {/if}
  {#if selection.qualification}<p>Inicio calificado: {factTimeLabel(selection.qualification.at)}</p>
    <p class="case-multiline">
      {selection.qualification.statement} / {selection.qualification.locator}
    </p>{/if}
  <p>
    Calendario: {value.calculation.material.calendar
      ? `${value.calculation.material.calendar.title} / Revision ${definition.input.calendar.revision}`
      : 'Sin calendario seleccionado'}
  </p>
  <p>Cantidad ordenada: {definition.input.ordered_quantity ?? 'Ausente'}</p>
  <details>
    <summary>Declaraciones de aplicabilidad y condiciones</summary>
    <p class="case-multiline">{definition.input.qualification.statement}</p>
    <p>Localizador: {definition.input.qualification.locator}</p>
    <p>El &#225;mbito aplica: {declaration(definition.input.qualification.scope_applies)}</p>
    <p>
      Existe incidencia sin resolver: {declaration(
        definition.input.qualification.unresolved_incident,
      )}
    </p>
    {#each definition.input.qualification.conditions as condition, index}<p class="case-multiline">
        Condici&#243;n {index + 1}: {profile?.definition.conditions.find(
          (row) => row.id === condition.id,
        )?.statement || condition.id}
        / {declaration(condition.applies)} / {condition.locator}
      </p>{/each}
  </details>
  <p>Atenci&#243;n: {value.attention.status === 'pending' ? 'Pendiente' : 'Declarada'}</p>
  {#if value.attention.status === 'recorded'}<p>{factTimeLabel(value.attention.occurred_at)}</p>
    <p class="case-multiline">{value.attention.statement} / {value.attention.locator}</p>{/if}
  <DeadlineCalculation calculation={value.calculation} />
</div>
