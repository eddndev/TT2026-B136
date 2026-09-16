<script>
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import {
    factDeclarationLabel,
    classLabels,
    characterLabels,
    mediumLabels,
    contextLabels,
    outcomeLabels,
  } from './fact-field-labels.mjs';
  import FactPersonValue from './FactPersonValue.svelte';
  import FactProvenanceValue from './FactProvenanceValue.svelte';
  export let values, family;
</script>

<dl class="case-values fact-values">
  {#if family === 'resolution'}
    <div>
      <dt>Clase declarada</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.class, classLabels)}</dd>
    </div>
    <div>
      <dt>Emisor declarado</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.issuer)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo de emisi&#243;n</dt>
      <dd>{factTimeLabel(values.issued_at)}</dd>
    </div>
  {:else}
    <div class="case-value-wide">
      <dt>Resoluci&#243;n vinculada</dt>
      <dd>
        Revisi&#243;n {values.resolution.revision}
        <details>
          <summary>Identidad de la resoluci&#243;n</summary><code>{values.resolution.id}</code>
        </details>
      </dd>
    </div>
    <div>
      <dt>Car&#225;cter declarado</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.character, characterLabels)}</dd>
    </div>
    <div>
      <dt>Medio declarado</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.medium, mediumLabels)}</dd>
    </div>
    <div>
      <dt>Contexto declarado</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.context, contextLabels)}</dd>
    </div>
    <div>
      <dt>Resultado declarado</dt>
      <dd class="case-multiline">{factDeclarationLabel(values.outcome, outcomeLabels)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo de pr&#225;ctica</dt>
      <dd>{factTimeLabel(values.practiced_at)}</dd>
    </div>
    <div class="case-value-wide">
      <dt>Tiempo de recepci&#243;n</dt>
      <dd>{values.received_at ? factTimeLabel(values.received_at) : 'No registrado'}</dd>
    </div>
  {/if}
  <div>
    <dt>Subtipo declarado</dt>
    <dd>{values.subtype || 'No registrado'}</dd>
  </div>
  <div class="case-value-wide">
    <dt>Resumen declarado</dt>
    <dd class="case-multiline">{values.summary}</dd>
  </div>
</dl>
{#if family === 'notification'}
  <FactPersonValue value={values.intended_recipient} label="Destinatario declarado" />
  <FactPersonValue value={values.actual_receiver} label="Receptor material" />
  <section class="case-comparison">
    <h4>Representaci&#243;n declarada</h4>
    {#if values.representation.kind === 'not_recorded'}<p class="case-multiline">
        No registrada: {values.representation.reason}
      </p>
    {:else}<FactPersonValue
        value={values.representation.represented}
        label="Persona representada"
        declared={false}
      />
      <FactPersonValue
        value={values.representation.representative}
        label="Persona representante"
        declared={false}
      />
      <p class="case-multiline">Alcance: {values.representation.scope}</p>
      <FactProvenanceValue
        value={values.representation.provenance}
        label={'Procedencia de la representaci\u00f3n'}
      />
    {/if}
  </section>
  {#if values.stated_effect}<section class="case-comparison">
      <h4>Efecto expresamente declarado</h4>
      <p>{factTimeLabel(values.stated_effect.at)}</p>
      <p class="case-multiline">{values.stated_effect.statement}</p>
      <p>Localizador en la procedencia principal: {values.stated_effect.locator}</p>
      <p class="hint">
        Declaraci&#243;n de la fuente, sin calificaci&#243;n de eficacia jur&#237;dica.
      </p>
    </section>{:else}<p class="hint">No se registr&#243; un efecto expresamente declarado.</p>{/if}
{/if}
<FactProvenanceValue value={values.provenance} />
