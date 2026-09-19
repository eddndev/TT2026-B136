<script>
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { factDeclarationLabel } from './fact-field-labels.mjs';
  import { resourceKinds, resourceActKinds } from '../lib/procedural-resource-values.mjs';
  export let values,
    act = false;
  const modeLabel = (value) =>
    value.kind === 'known'
      ? { oral: 'Oral', written: 'Escrita' }[value.value] || 'Sin declarar'
      : factDeclarationLabel(value);
  $: supports = act ? values.evidence : [values.resolution_evidence];
</script>

<dl class="case-values fact-values">
  <div>
    <dt>{act ? 'Tipo de acto' : 'Tipo de recurso'}</dt>
    <dd>{(act ? resourceActKinds : resourceKinds)[values.kind]}</dd>
  </div>
  <div>
    <dt>Modalidad declarada</dt>
    <dd class="case-multiline">{modeLabel(values.mode)}</dd>
  </div>
  {#if act}
    <div>
      <dt>Autoridad del acto</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.authority)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo declarado del acto</dt>
      <dd>{factTimeLabel(values.occurred_at)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Declaraci&#243;n del acto</dt>
      <dd class="case-multiline">{values.statement}</dd>
    </div>
  {:else}
    <div class="case-value-wide">
      <dt>Titulo organizativo</dt>
      <dd>{values.title}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Resoluci&#243;n impugnada</dt>
      <dd>
        Revisi&#243;n exacta {values.resolution.revision}
        <details>
          <summary>Identidad de la resoluci&#243;n</summary><code>{values.resolution.id}</code>
        </details>
      </dd>
    </div>
    <div>
      <dt>Referencia de la resoluci&#243;n</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.resolution_reference)}</dd>
    </div>
    <div>
      <dt>Autoridad emisora</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.issuing_authority)}</dd>
    </div>
    <div>
      <dt>Autoridad receptora</dt>
      <dd class="case-multiline">
        {values.receiving_authority
          ? factDeclarationLabel(values.receiving_authority)
          : 'No registrada'}
      </dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo de la resoluci&#243;n</dt>
      <dd>{factTimeLabel(values.resolution_at)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo de notificaci&#243;n</dt>
      <dd>{values.notification_at ? factTimeLabel(values.notification_at) : 'No registrado'}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Parte impugnada</dt>
      <dd class="case-multiline">{values.challenged_part}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Motivos del recurso</dt>
      <dd class="case-multiline">{values.grounds}</dd>
    </div>
  {/if}
</dl>
{#if !act}
  <section class="case-comparison" aria-label="Personas recurrentes capturadas">
    <h4>Personas recurrentes capturadas</h4>
    {#each values.appellants as person, index}
      <div class="hearing-result-picker-row">
        <p><strong>{index + 1}. {person.name}</strong></p>
        <p class="case-multiline">Rol: {factDeclarationLabel(person.role)}</p>
        {#if person.participant}
          <p>Ficha del expediente / Revisi&#243;n {person.participant.revision}</p>
          <details>
            <summary>Referencia de recurrente {index + 1}</summary><code
              >{person.participant.id}</code
            >
          </details>
        {:else}<p>Sin ficha vinculada.</p>{/if}
      </div>
    {/each}
  </section>
{/if}
<section
  class="case-comparison"
  aria-label={act ? 'Soportes del acto' : 'Soporte de la resoluci\u00f3n impugnada'}
>
  <h4>{act ? 'Soportes del acto' : 'Soporte de la resoluci\u00f3n impugnada'}</h4>
  {#each supports as support}
    <p>Versi&#243;n exacta {support.version}</p>
    <p class="case-multiline">Localizador: {support.locator}</p>
    <details>
      <summary>Identidad y digest del soporte</summary>
      <p><code>{support.document_id}</code></p>
      <p><code>{support.digest}</code></p>
    </details>
  {/each}
</section>
