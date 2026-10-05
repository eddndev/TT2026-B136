<script>
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import FactSources from './FactSources.svelte';
  import ParticipantSubjectSummary from './ParticipantSubjectSummary.svelte';
  import { measureKinds, measureActions, measureTimeLabel } from '../lib/measure-presentation.mjs';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let value,
    confirmed = false,
    disabled = false;
  const actions = {
    correct: 'Corregir captura',
    entered_in_error: 'Marcar registro por error',
    replace_entered_in_error: 'Corregir identidad registrada',
  };
  $: command = value.command;
  $: result = value.result;
  $: context = value.context;
</script>

<section class="case-comparison" aria-label="Revision de rectificacion">
  <h3>Revision de rectificacion</h3>
  <p class="notice">
    Esta rectificacion administrativa conserva la decision judicial historica y su procedencia.
    Revisa el registro seleccionado y el resultado propuesto antes de confirmar.
  </p>
  <dl class="case-values">
    <div>
      <dt>Accion administrativa</dt>
      <dd>{actions[command.action.kind]}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Motivo de rectificacion</dt>
      <dd class="case-multiline">{command.reason}</dd>
    </div>
    <div>
      <dt>Clase de medida</dt>
      <dd>{measureKinds[result.values.kind]}</dd>
    </div>
    <div>
      <dt>Ultima accion judicial</dt>
      <dd>{measureActions[result.last_action]}</dd>
    </div>
    <div>
      <dt>Revision seleccionada</dt>
      <dd>{command.target.revision}</dd>
    </div>
    <div>
      <dt>Revision administrativa propuesta</dt>
      <dd>{result.revision}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Medida seleccionada</dt>
      <dd><code>{command.target.id}</code></dd>
    </div>
    <div class="case-value-wide">
      <dt>Captura exacta seleccionada</dt>
      <dd><code>{command.target.capture_digest}</code></dd>
    </div>
  </dl>
  <ParticipantSubjectSummary record={result.sources.subject} />
  {#if command.action.kind === 'correct'}
    <h4>Valores rectificados</h4>
    <dl class="case-values">
      <div class="case-value-wide">
        <dt>Condiciones</dt>
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
        <dt>Declaracion de vigencia</dt>
        <dd class="case-multiline">{result.values.validity.statement}</dd>
      </div>
      <div class="case-value-wide">
        <dt>Texto de supervision</dt>
        <dd class="case-multiline">{command.action.values.supervision_text}</dd>
      </div>
    </dl>
  {:else}
    <p>El registro seleccionado quedara marcado como registrado por error.</p>
  {/if}
  {#if value.replacement}
    <section class="case-comparison" aria-label="Identidad de reemplazo propuesta">
      <h4>Registro de reemplazo</h4>
      <ParticipantSubjectSummary record={value.replacement.sources.subject} />
      <p>
        Nueva medida: <code>{value.replacement.id}</code> / Revision {value.replacement.revision}
      </p>
      <p>
        Vinculo propuesto desde <code>{result.id}</code>, revision {result.revision}, al registro de
        reemplazo <code>{value.replacement.id}</code>.
      </p>
      <p>Ambos registros se conservaran en la misma operacion administrativa.</p>
    </section>
  {/if}
  <FactSources
    sources={{
      resolution: null,
      participants: [],
      hearing_results: [],
      direct_supports: [value.support],
    }}
  />
  <details>
    <summary>Contexto exacto de la rectificacion</summary>
    <FactAdministrativeCapture value={context.administration} label="Administracion seleccionada" />
    <p>Etapa: {stageLabels[context.stage.stage]} / Revision {context.stage.stage_revision}</p>
    <FactAdministrativeCapture
      value={context.stage_administration}
      label="Administracion de origen de la etapa"
    />
    <p>Contexto: <code>{context.context_digest}</code></p>
  </details>
  <p>Autor: {value.actor.email} / {value.actor.role}</p>
  <details>
    <summary>Identidades y huellas de la rectificacion</summary>
    <p>Expediente: <code>{value.case_id}</code></p>
    <p>Autor: <code>{value.actor.id}</code></p>
    <p>Operacion: <code>{command.operation_id}</code></p>
    <p>Decision judicial de origen: <code>{result.judicial_origin.decision_id}</code></p>
    <p>Operacion judicial de origen: <code>{result.judicial_origin.operation_id}</code></p>
    <p>Huella del envio: <code>{value.submission_digest}</code></p>
    <p>Huella de la revision preparada: <code>{value.review_digest}</code></p>
  </details>
  <label class="checkbox-row">
    <input type="checkbox" bind:checked={confirmed} {disabled} />
    Reconozco la rectificacion y su alcance administrativo
  </label>
</section>
