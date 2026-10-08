<script>
  import FactSources from './FactSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import { hearingModalities, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import { stageLabels } from '../lib/case-stages.mjs';
  import {
    precautionaryHearingPurposes,
    precautionaryHearingActions,
  } from '../lib/precautionary-hearing-presentation.mjs';
  export let value;
  $: values = value.resolved_values;
  $: change = value.command.change;
</script>

<section class="case-comparison" aria-label="Revision de convocatoria cautelar">
  <h3>
    {precautionaryHearingActions[change.action]} / {precautionaryHearingPurposes[values.purpose]}
  </h3>
  <p class="notice">
    Preparaci&#243;n pendiente de confirmar. Revisa la convocatoria y sus fuentes seleccionadas.
  </p>
  <p>Revisi&#243;n propuesta: {value.result_revision} / {hearingStatus[value.status]}</p>
  <dl class="case-values">
    <div>
      <dt>Programaci&#243;n declarada</dt>
      <dd>{hearingTimeLabel(values.scheduled_at)}</dd>
    </div>
    <div>
      <dt>Modalidad</dt>
      <dd>{hearingModalities[values.modality]}</dd>
    </div>
    <div>
      <dt>Sede o enlace</dt>
      <dd>{values.venue}</dd>
    </div>
    {#if values.note}<div class="case-value-wide">
        <dt>Nota</dt>
        <dd class="case-multiline">{values.note}</dd>
      </div>{/if}
  </dl>
  <h4>Base de se&#241;alamiento</h4>
  <p class="case-multiline">{values.scheduling_basis.statement}</p>
  <p class="case-multiline">Localizador: {values.scheduling_basis.locator}</p>
  {#if change.reason}<h4>Motivo del cambio</h4>
    <p class="case-multiline">{change.reason}</p>{/if}
  {#if values.review_targets.length}
    <section class="case-comparison" aria-label="Medidas seleccionadas para revision">
      <h4>Medidas seleccionadas para revisi&#243;n</h4>
      {#each values.review_targets as target (target.id)}
        <details>
          <summary>Medida / Revisi&#243;n {target.revision}</summary>
          <p>Identidad: <code>{target.id}</code></p>
          <p>Captura: <code>{target.capture_digest}</code></p>
        </details>
      {/each}
    </section>
  {/if}
  <FactSources
    sources={{
      resolution: null,
      participants: value.participants.map((person) => person.overview),
      hearing_results: [],
      direct_supports: [value.sources.support],
    }}
  />
  <details>
    <summary>Contexto de la convocatoria</summary>
    <FactAdministrativeCapture
      value={value.scheduling_context.administration}
      label="Administracion de la convocatoria"
    />
    <p>
      Etapa: {stageLabels[value.scheduling_context.stage.stage]} / Revisi&#243;n {value
        .scheduling_context.stage.stage_revision}
    </p>
    <FactAdministrativeCapture
      value={value.observed_context.administration}
      label="Administracion observada al preparar"
    />
    <p>
      Etapa observada: {stageLabels[value.observed_context.stage.stage]} / Revisi&#243;n {value
        .observed_context.stage.stage_revision}
    </p>
  </details>
  <p>Autor: {value.actor.email}</p>
  <details>
    <summary>Identidades y huellas de preparaci&#243;n</summary>
    <p>Audiencia: <code>{value.command.hearing_id}</code></p>
    <p>Operaci&#243;n: <code>{value.command.operation_id}</code></p>
    {#if change.action !== 'schedule'}
      <p>Revisi&#243;n anterior esperada: {change.expected_revision}</p>
      <p>Captura anterior esperada: <code>{change.expected_capture_digest}</code></p>
    {/if}
    <p>Huella del env&#237;o: <code>{value.submission_digest}</code></p>
    <p>Huella de la revisi&#243;n preparada: <code>{value.review_digest}</code></p>
  </details>
</section>
