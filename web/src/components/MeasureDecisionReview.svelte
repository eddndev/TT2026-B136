<script>
  import FactSources from './FactSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import { measureKinds, measureActions, measureTimeLabel } from '../lib/measure-presentation.mjs';
  import { hearingModalities, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
  import { precautionaryHearingPurposes } from '../lib/precautionary-hearing-presentation.mjs';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let value,
    confirmed = false;
  $: review = value.review;
  $: command = review.command;
  $: values = command.values;
  $: context = review.material.context;
  $: anchor = review.material.anchor;
  $: anchorValues =
    anchor?.kind === 'initial' ? anchor.hearing.values : anchor?.capture.review.resolved_values;
  $: anchorParticipants =
    anchor?.kind === 'initial'
      ? anchor.hearing.participants.map((row) => ({
          ...row.overview,
          values_digest: row.values_digest,
        }))
      : anchor?.capture.review.participants.map((row) => ({
          ...row.overview,
          values_digest: row.snapshot.values_digest,
        }));
  $: anchorSupport =
    anchor?.kind === 'initial' ? anchor.hearing.support : anchor?.capture.review.sources.support;
</script>

<section class="case-comparison" aria-label="Revision de decision cautelar">
  <h3>{confirmed ? 'Decision declarada registrada' : 'Revision de la decision declarada'}</h3>
  {#if !confirmed}<p class="notice">
      Preparaci&#243;n pendiente de confirmar. Revisa la decisi&#243;n, sus efectos y las fuentes
      seleccionadas.
    </p>{/if}
  <dl class="case-values">
    <div class="case-value-wide">
      <dt>Autoridad declarada</dt>
      <dd class="case-multiline">{values.authority}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Momento declarado de la decisi&#243;n</dt>
      <dd>{measureTimeLabel(values.declared_at)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Justificaci&#243;n declarada</dt>
      <dd class="case-multiline">{values.justification}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Localizador en el soporte</dt>
      <dd class="case-multiline">{values.locator}</dd>
    </div>
  </dl>
  <FactSources
    sources={{
      resolution: null,
      participants: [],
      hearing_results: [],
      direct_supports: [review.material.support],
    }}
  />
  {#if command.outcome.kind === 'no_measure_change'}
    <section class="case-comparison" aria-label="Resultado sin cambios de medidas">
      <h4>Sin cambios de medidas</h4>
      <p class="case-multiline">{command.outcome.statement}</p>
    </section>
  {:else}
    <h4>Efectos declarados</h4>
    {#each review.results as result (result.id)}
      {@const origin = value.family === 'g1' ? result.origin : result.judicial_origin}
      <section class="case-comparison" aria-label="Medida en la decision">
        <h4>{measureActions[result.action]} / {measureKinds[result.values.kind]}</h4>
        <p><strong>{result.projection.subject.display_name}</strong></p>
        <p>{confirmed ? 'Revision registrada' : 'Revision propuesta'}: {result.revision}</p>
        <dl class="case-values">
          <div class="case-value-wide">
            <dt>Condiciones declaradas</dt>
            <dd class="case-multiline">{result.values.conditions}</dd>
          </div>
          <div>
            <dt>Inicio declarado</dt>
            <dd>{measureTimeLabel(result.values.validity.start)}</dd>
          </div>
          <div>
            <dt>Fin declarado</dt>
            <dd>
              {result.values.validity.end
                ? measureTimeLabel(result.values.validity.end)
                : 'Sin fin declarado'}
            </dd>
          </div>
          <div class="case-value-wide">
            <dt>Vigencia declarada</dt>
            <dd class="case-multiline">{result.values.validity.statement}</dd>
          </div>
          <div class="case-value-wide">
            <dt>Supervisi&#243;n declarada</dt>
            <dd class="case-multiline">
              {result.values.supervision.kind === 'unknown'
                ? result.values.supervision.reason
                : result.values.supervision.statement}
            </dd>
          </div>
        </dl>
        {#if result.projection.supervisor}<FactSources
            sources={{
              resolution: null,
              participants: [
                {
                  ...result.projection.supervisor.overview,
                  values_digest: result.projection.supervisor.snapshot.values_digest,
                },
              ],
              hearing_results: [],
              direct_supports: [],
            }}
          />{/if}
        <details>
          <summary>Identidad exacta del sujeto</summary>
          <ParticipantSubjectSummary record={result.sources.subject} />
        </details>
        <details>
          <summary>Referencia y procedencia de la medida</summary>
          <p>Medida: <code>{result.id}</code> / Revisi&#243;n {result.revision}</p>
          {#if result.previous}
            <p>Revisi&#243;n anterior seleccionada: {result.previous.revision}</p>
            <p>Medida anterior: <code>{result.previous.id}</code></p>
            <p>Captura anterior: <code>{result.previous.capture_digest}</code></p>
          {:else}<p>Nueva identidad de medida.</p>{/if}
          <p>Grupo de efecto: <code>{result.effect_key}</code></p>
          <p>Decisi&#243;n judicial de origen: <code>{origin.decision_id}</code></p>
          <p>Operaci&#243;n judicial de origen: <code>{origin.operation_id}</code></p>
        </details>
      </section>
    {/each}
  {/if}
  <section class="case-comparison" aria-label="Audiencia de origen de la decision">
    <h4>Audiencia de origen</h4>
    {#if anchor}
      <p>
        {anchor.kind === 'initial'
          ? 'Audiencia inicial'
          : precautionaryHearingPurposes[anchorValues.purpose]}
        / Revisi&#243;n {command.anchor.revision}
      </p>
      <p>
        {hearingStatus[
          anchor.kind === 'initial' ? anchor.hearing.status : anchor.capture.review.status
        ]}
        en la revisi&#243;n seleccionada
      </p>
      <p>
        {hearingTimeLabel(anchorValues.scheduled_at)} / {hearingModalities[anchorValues.modality]}
      </p>
      <p class="case-multiline">{anchorValues.venue}</p>
      {#if anchorValues.note}<p class="case-multiline">{anchorValues.note}</p>{/if}
      {#if anchorValues.scheduling_basis}
        <p class="case-multiline">{anchorValues.scheduling_basis.statement}</p>
        <p>Localizador: {anchorValues.scheduling_basis.locator}</p>
      {/if}
      <FactSources
        sources={{
          resolution: null,
          participants: anchorParticipants,
          hearing_results: [],
          direct_supports: anchorSupport ? [anchorSupport] : [],
        }}
      />
      <details>
        <summary>Referencia exacta de la audiencia</summary>
        <p>Audiencia: <code>{command.anchor.hearing_id}</code></p>
        {#if anchor.kind === 'initial'}
          <p>Valores: <code>{command.anchor.values_digest}</code></p>
          <p>Recibo: <code>{command.anchor.submission_digest}</code></p>
        {:else}<p>Captura: <code>{command.anchor.capture_digest}</code></p>{/if}
      </details>
    {:else}<p>Decisi&#243;n independiente, sin audiencia de origen seleccionada.</p>{/if}
  </section>
  <details>
    <summary>Contexto exacto de la decisi&#243;n</summary>
    <FactAdministrativeCapture value={context.administration} label="Administracion seleccionada" />
    <p>Etapa: {stageLabels[context.stage.stage]} / Revisi&#243;n {context.stage.stage_revision}</p>
    <FactAdministrativeCapture
      value={context.stage_administration}
      label="Administracion de origen de la etapa"
    />
    <p>Contexto: <code>{context.context_digest}</code></p>
  </details>
  <p>Autor: {review.actor.email}</p>
  <details>
    <summary>Identidades y huellas de la decisi&#243;n</summary>
    <p>Expediente: <code>{review.case_id}</code></p>
    <p>Decisi&#243;n: <code>{command.decision_id}</code></p>
    <p>Operaci&#243;n: <code>{command.operation_id}</code></p>
    <p>Huella del env&#237;o: <code>{review.submission_digest}</code></p>
    <p>Huella de la revisi&#243;n preparada: <code>{review.review_digest}</code></p>
  </details>
</section>
