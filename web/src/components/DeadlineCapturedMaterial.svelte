<script>
  import DeadlineReference from './DeadlineReference.svelte';
  import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { deadlineDeclarationLabel, deadlinePurposes } from './deadline-view-labels.mjs';
  export let api, record, ondenied;
  $: material = record.calculation.material;
  $: input = record.definition.input;
</script>

<section aria-label="Insumos capturados del plazo">
  <h3>Declaraciones utilizadas</h3>
  <p class="case-multiline">{input.qualification.statement}</p>
  <p>Localizador: {input.qualification.locator}</p>
  <dl class="case-values">
    <div>
      <dt>El perfil aplica al expediente</dt>
      <dd>{deadlineDeclarationLabel(input.qualification.scope_applies)}</dd>
    </div>
    <div>
      <dt>Existe una incidencia pendiente</dt>
      <dd>{deadlineDeclarationLabel(input.qualification.unresolved_incident)}</dd>
    </div>
    <div>
      <dt>Duraci&#243;n concedida</dt>
      <dd>{input.ordered_quantity ?? 'No declarada'}</dd>
    </div>
  </dl>
  {#each input.qualification.conditions as condition}<div class="case-comparison">
      <p>Condici&#243;n <code>{condition.id}</code></p>
      <p>{deadlineDeclarationLabel(condition.applies)}</p>
      <p>{condition.locator}</p>
    </div>{/each}
  {#if input.selection.qualification}{@const q = input.selection.qualification}
    <div class="case-comparison">
      <h4>{deadlinePurposes[q.purpose]}</h4>
      <p>{factTimeLabel(q.at)}</p>
      <p class="case-multiline">{q.statement}</p>
      <p>{q.locator}</p>
    </div>{/if}
  <h3>Fuentes y calendarios conservados</h3>
  <p class="hint">
    La selecci&#243;n y la revisi&#243;n m&#225;s reciente observada se conservan por separado. Una
    modificaci&#243;n posterior no cambia esta captura.
  </p>
  {#if material.source}<DeadlineReference
      {api}
      caseId={record.case_id}
      captured={material.source}
      label="Fuente seleccionada"
      {ondenied}
    />
  {:else}<p>Fuente desconocida: {input.selection.source.reason}</p>{/if}
  {#if material.source_head}<DeadlineReference
      {api}
      caseId={record.case_id}
      captured={material.source_head}
      label="Fuente observada al registrar"
      {ondenied}
    />{/if}
  {#if material.calendar}<DeadlineReference
      {api}
      caseId={record.case_id}
      kind="calendar"
      captured={material.calendar}
      label="Calendario seleccionado"
      {ondenied}
    />
  {:else}<p>Sin calendario seleccionado.</p>{/if}
  {#if material.calendar_head}<DeadlineReference
      {api}
      caseId={record.case_id}
      kind="calendar"
      captured={material.calendar_head}
      label="Calendario observado al registrar"
      {ondenied}
    />{/if}
  <details>
    <summary>Administraci&#243;n del expediente al registrar</summary>
    <p>
      {material.administration.title} / {material.administration.reference ||
        'Sin referencia interna'}
    </p>
    {#if material.administration.kind === 'recorded'}<p>
        Revisi&#243;n {material.administration.revision} / {material.administration.changed_by
          .email}
      </p>
      <p>{deadlineInstantLabel(material.administration.changed_at)}</p>
      <code>{material.administration.values_digest}</code>
    {:else}<p>Ficha original sin revisi&#243;n administrativa registrada.</p>{/if}
    <p>
      {material.administration.status === 'closed'
        ? 'Cerrado administrativamente'
        : 'Activo administrativamente'}
    </p>
  </details>
</section>
