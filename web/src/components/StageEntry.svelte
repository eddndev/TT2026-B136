<script>
  import StageValues from './StageValues.svelte';
  import { stageLabels } from '../lib/case-stages.mjs';
  export let record;
</script>

<article class="stage-entry">
  <span class="badge neutral"
    >{record.kind === 'initial'
      ? 'Registro inicial'
      : record.values.kind === 'adoption'
        ? 'Adopci\u00f3n'
        : 'Transici\u00f3n'}</span
  >
  <h3>{stageLabels[record.stage]} / Revisi&#243;n {record.stage_revision}</h3>
  {#if record.kind === 'initial'}<p>
      Investigaci&#243;n registrada al crear el expediente. No se atribuyen actos o soportes
      anteriores.
    </p>
  {:else}
    {#if record.from_stage}<p>
        Desde {stageLabels[record.from_stage]} hacia {stageLabels[record.stage]}.
      </p>{/if}
    <h4>Actos declarados</h4>
    <StageValues values={record.values} supports={record.supports} />
  {/if}
  <p class="hint">
    Registrado en el sistema por {record.recorded_by.email} /
    <time datetime={record.recorded_at} title={record.recorded_at}
      >{new Date(record.recorded_at).toLocaleString('es-MX')}</time
    >
  </p>
  <details>
    <summary>Trazabilidad del registro de etapa</summary>
    <p class="hint">Revisi&#243;n administrativa capturada: {record.administration_revision}.</p>
    <p><code>{record.administration_digest}</code></p>
    {#if record.values_digest}<p><code>{record.values_digest}</code></p>{/if}
  </details>
</article>
