<script>
  import { measureKinds, measureActions, measureTimeLabel } from '../lib/measure-presentation.mjs';
  export let value;
  $: capture = value.record.capture;
  $: result = capture.result;
  $: values = result.values;
</script>

<section class="case-comparison" aria-label="Registro exacto de medida">
  <h4>{measureKinds[values.kind]}</h4>
  <p><strong>{result.projection.subject.display_name}</strong></p>
  <p>Ultima acci&#243;n judicial registrada: {measureActions[value.last_action]}</p>
  {#if value.validity === 'entered_in_error'}<p class="notice">
      Captura marcada como registrada por error.
    </p>{/if}
  <dl class="case-values">
    <div class="case-value-wide">
      <dt>Condiciones declaradas</dt>
      <dd class="case-multiline">{values.conditions}</dd>
    </div>
    <div>
      <dt>Inicio declarado</dt>
      <dd>{measureTimeLabel(values.validity.start)}</dd>
    </div>
    <div>
      <dt>Fin declarado</dt>
      <dd>{values.validity.end ? measureTimeLabel(values.validity.end) : 'Sin fin declarado'}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Vigencia declarada</dt>
      <dd class="case-multiline">{values.validity.statement}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Supervisi&#243;n declarada</dt>
      <dd class="case-multiline">
        {values.supervision.kind === 'unknown'
          ? values.supervision.reason
          : values.supervision.statement}
      </dd>
    </div>
  </dl>
  <p class="hint">
    La captura conserva lo declarado; no acredita cumplimiento ni determina vigencia por el reloj.
  </p>
  <p>Medida: <code>{value.reference.id}</code> / Revisi&#243;n {value.reference.revision}</p>
  <p>Captura: <code>{value.reference.capture_digest}</code></p>
  <p>
    Registrado por {capture.actor.email} /
    <time datetime={capture.recorded_at}>{capture.recorded_at}</time>
  </p>
</section>
