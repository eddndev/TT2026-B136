<script>
  import { onMount } from 'svelte';
  import ResourceHearingValues from './ResourceHearingValues.svelte';
  import ResourceHearingSources from './ResourceHearingSources.svelte';
  import FactAdministrativeCapture from './FactAdministrativeCapture.svelte';
  import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
  import '../styles/hearings.css';
  import '../styles/procedural-facts.css';
  export let value, caseRecord, onclose;
  let heading;
  $: hearing = value.hearing;
  onMount(() => heading?.focus());
</script>

<section class="card hearing-detail fact-detail" aria-label="Detalle de audiencia de recurso">
  <div class="section-heading">
    <h2 tabindex="-1" bind:this={heading}>{resourceHearingKinds[hearing.values.kind]}</h2>
    <button class="secondary" onclick={onclose}>Cerrar detalle de audiencia</button>
  </div>
  <p><strong>{caseRecord.title}</strong> / {caseRecord.reference}</p>
  {#if caseRecord.administration?.administrative_status === 'closed'}
    <p class="notice">
      Expediente cerrado administrativamente. Consulta hist&#243;rica disponible.
    </p>
  {/if}
  <p class="hint">
    Audiencia de recurso / Revisi&#243;n {hearing.revision} consultada exactamente.
  </p>
  <p class="notice">
    Esta captura hist&#243;rica conserva el horario y las fuentes seleccionadas al registrar. No
    determina procedencia jur&#237;dica ni asistencia.
  </p>
  <ResourceHearingValues values={hearing.values} />
  <ResourceHearingSources sources={hearing.sources} />
  <details class="hearing-provenance">
    <summary>Autor y recibo original</summary>
    <p>
      {hearing.recorded_by.email} /
      <time datetime={hearing.recorded_at}>{hearing.recorded_at}</time>
    </p>
    <FactAdministrativeCapture
      value={hearing.recorded_administration}
      label={'Administracion al registrar'}
    />
    <p>
      Cabeza del recurso observada al registrar: revisi&#243;n {hearing.recorded_resource_head
        .revision}.
    </p>
    <p>Audiencia: <code>{hearing.id}</code></p>
    <p>Expediente: <code>{hearing.case_id}</code></p>
    <p>Operaci&#243;n: <code>{hearing.operation_id}</code></p>
    <p>Recibo: <code>{hearing.submission_digest}</code></p>
    <p>Captura: <code>{hearing.capture_digest}</code></p>
    <h3>Asociaci&#243;n original / Revisi&#243;n {value.association.revision}</h3>
    <p>Identidad: <code>{value.origin.association_id}</code></p>
    <p>Captura: <code>{value.association.receipt.capture_digest}</code></p>
    <p class="hint">
      La asociaci&#243;n pertenece al registro original. No declara su estado actual.
    </p>
  </details>
</section>
