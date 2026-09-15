<script>
  import CaseValues from './CaseValues.svelte';
  import CaseEditor from './CaseEditor.svelte';
  import CaseStatus from './CaseStatus.svelte';
  import CaseHistory from './CaseHistory.svelte';
  import { manageCase } from '../lib/case-administration.mjs';
  import { caseState } from '../lib/case-state.mjs';
  export let api, scoped, user, record, onupdate, ondenied, onrefresh, onnavigate;
  const state = caseState();
  let editing = null,
    statusDialog,
    history = false;
  $: current = record.administration;
  function confirmed(result) {
    onupdate(result);
    editing = null;
  }
</script>

<div class="page-heading">
  <div>
    <span class="eyebrow">DATOS DEL EXPEDIENTE</span>
    <h1>Resumen del expediente</h1>
    <p>Ficha, administraci&#243;n y registro inicial de etapa.</p>
  </div>
  <button class="secondary" onclick={onrefresh}>Actualizar resumen</button>
</div>
<section class="card case-summary">
  <div class="section-heading">
    <h2>{current.profile ? 'Ficha penal completa' : 'Ficha penal pendiente'}</h2>
    {#if manageCase(user.role)}<div class="action-row">
        <button
          class="secondary"
          disabled={$state.closed || !!editing}
          onclick={() => (editing = current.profile ? 'profile' : 'basic')}
          >{current.profile ? 'Editar ficha penal' : 'Editar datos b\u00e1sicos'}</button
        >
        {#if !current.profile}<button
            class="primary"
            disabled={$state.closed || !!editing}
            onclick={() => (editing = 'profile')}>Completar ficha penal</button
          >{/if}
        <button class="text-button" onclick={() => statusDialog.open()}
          >{current.administrative_status === 'closed'
            ? 'Reactivar expediente'
            : 'Cerrar administrativamente'}</button
        >
      </div>{/if}
  </div>
  {#if !current.profile}<p class="hint">
      Este expediente a&#250;n no tiene una ficha penal completa. La referencia interna conserva su
      significado original.
    </p>{/if}
  <CaseValues record={current} />
  <p class="hint">
    Creado: <time datetime={record.created_at} title={record.created_at}
      >{new Date(record.created_at).toLocaleString('es-MX')}</time
    >
  </p>
  {#if current.revision}<p class="hint">
      Revisi&#243;n {current.revision} / {current.changed_by.email} /
      <time datetime={current.changed_at} title={current.changed_at}
        >{new Date(current.changed_at).toLocaleString('es-MX')}</time
      >
    </p>
  {:else}<p class="hint">
      Datos originales; a&#250;n no hay una revisi&#243;n administrativa registrada.
    </p>{/if}
  <button class="text-button" aria-expanded={history} onclick={() => (history = !history)}
    >{history ? 'Ocultar historial administrativo' : 'Ver historial administrativo'}</button
  >
  {#if history}{#key current.revision}<CaseHistory api={scoped} {ondenied} />{/key}{/if}
</section>
{#if editing}<CaseEditor
    {api}
    {record}
    complete={editing === 'profile'}
    disabled={$state.closed}
    onconfirmed={confirmed}
    onobserved={onupdate}
    oncancel={() => (editing = null)}
    {ondenied}
  />{/if}
<section class="card case-stage">
  <span class="eyebrow">REGISTRO INICIAL DE ETAPA</span>
  {#if record.initial_stage}<h2>Investigaci&#243;n</h2>
    <p>Etapa registrada al crear el expediente. Este registro no acredita un acto judicial.</p>
    <p class="hint">
      Registrada por {record.initial_stage.recorded_by.email} /
      <time datetime={record.initial_stage.recorded_at} title={record.initial_stage.recorded_at}
        >{new Date(record.initial_stage.recorded_at).toLocaleString('es-MX')}</time
      >
    </p>
    <details>
      <summary>Trazabilidad del registro inicial</summary>
      <p class="hint">
        Revisi&#243;n inicial de etapa {record.initial_stage.stage_revision}; revisi&#243;n
        administrativa {record.initial_stage.administration_revision}.
      </p>
      <code>{record.initial_stage.administration_digest}</code>
    </details>
  {:else}<h2>Etapa sin registrar</h2>
    <p>Completar la ficha no crea un registro de etapa ni reconstruye actos anteriores.</p>{/if}
  <button class="text-button" onclick={() => onnavigate('stages')}>Consultar etapas</button>
</section>
{#if manageCase(user.role)}<CaseStatus
    bind:this={statusDialog}
    api={scoped}
    {record}
    onconfirmed={onupdate}
    onobserved={onupdate}
    {ondenied}
  />{/if}
