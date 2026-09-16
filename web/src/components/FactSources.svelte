<script>
  import { factTimeLabel } from '../lib/procedural-fact-time.mjs';
  import { resultOccurrence } from '../lib/hearing-result-errors.mjs';
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  import { factDeclarationLabel, classLabels } from './fact-field-labels.mjs';
  import HearingReference from './HearingReference.svelte';
  export let sources;
</script>

<section class="fact-sources" aria-label="Fuentes exactas de la captura">
  <h3>Fuentes exactas de la captura</h3>
  <p class="hint">
    Datos de las revisiones seleccionadas; no se actualizan a la versi&#243;n vigente de cada
    fuente.
  </p>
  {#if sources.resolution}{@const row = sources.resolution}
    <div class="case-comparison">
      <h4>Resoluci&#243;n de origen / Revisi&#243;n {row.revision}</h4>
      <p>{row.status === 'withdrawn' ? 'Retirada' : 'Registrada'} en esta revisi&#243;n</p>
      <p>{factDeclarationLabel(row.class, classLabels)}</p>
      <p>Emisor: {factDeclarationLabel(row.issuer)}</p>
      <p>{factTimeLabel(row.issued_at)}</p>
      <p class="case-multiline">{row.summary}</p>
      <details>
        <summary>Identidad y huellas de la resoluci&#243;n</summary>
        <p><code>{row.id}</code></p>
        <p>Expediente: <code>{row.case_id}</code></p>
        <p>Valores: <code>{row.values_digest}</code></p>
        <p>Recibo: <code>{row.submission_digest}</code></p>
      </details>
    </div>
  {/if}
  {#if sources.participants.length}<h4>Fichas e identidades hist&#243;ricas</h4>{/if}
  {#each sources.participants as row (`${row.id}:${row.revision}`)}
    <div class="case-comparison">
      <p><strong>{row.display_name}</strong> / Revisi&#243;n {row.revision}</p>
      <p>
        {row.kind ? profileLabel(row.kind) : row.procedural_role} / {row.directory_status ===
        'archived'
          ? 'Archivada'
          : 'Activa'} en esta revisi&#243;n
      </p>
      {#if row.organization}<p>{row.organization}</p>{/if}
      <HearingReference {row} />
      <details><summary>Expediente de la ficha</summary><code>{row.case_id}</code></details>
    </div>
  {/each}
  {#if sources.hearing_results.length}<h4>Resultados de audiencia hist&#243;ricos</h4>{/if}
  {#each sources.hearing_results as row (`${row.hearing_id}:${row.result_id}:${row.revision}:${row.agreement_id || ''}`)}
    <div class="case-comparison">
      <p>
        Revisi&#243;n {row.revision} / {row.status === 'withdrawn' ? 'Retirado' : 'Registrado'} en esta
        revisi&#243;n
      </p>
      <p>
        {resultOccurrence[row.occurrence]} / {factTimeLabel({
          ...row.event_time,
          precision: row.event_time.precision === 'instant' ? 'second' : 'date',
        })}
      </p>
      <p class="case-multiline">{row.summary}</p>
      {#if row.agreement}<h5>Acuerdo seleccionado</h5>
        <p class="case-multiline">{row.agreement.text}</p>{:else}<p class="hint">
          Sin acuerdo espec&#237;fico.
        </p>{/if}
      <details>
        <summary>Referencia y huellas del resultado</summary>
        <p>Expediente: <code>{row.case_id}</code></p>
        <p>Audiencia: <code>{row.hearing_id}</code></p>
        <p>Resultado: <code>{row.result_id}</code></p>
        {#if row.agreement_id}<p>Acuerdo: <code>{row.agreement_id}</code></p>{/if}
        <p>Valores: <code>{row.values_digest}</code></p>
        <p>Recibo: <code>{row.submission_digest}</code></p>
      </details>
    </div>
  {/each}
  {#if sources.direct_supports.length}<h4>Soportes documentales directos</h4>{/if}
  {#each sources.direct_supports as row (`${row.document_id}:${row.version}`)}
    <div class="case-comparison">
      <p>
        <strong>{row.name}</strong> / Versi&#243;n {row.version} / {row.format === 'pdf'
          ? 'PDF'
          : 'DOCX'}
      </p>
      <p class="hint">Admitido con la pol&#237;tica PDF/DOCX versi&#243;n 1.</p>
      <details>
        <summary>Referencia y huella del soporte</summary>
        <p><code>{row.document_id}</code></p>
        <p>Huella: <code>{row.digest}</code></p>
      </details>
    </div>
  {/each}
  {#if !sources.resolution && !sources.participants.length && !sources.hearing_results.length && !sources.direct_supports.length}<p
    >
      Sin referencias a fuentes del expediente en esta captura.
    </p>{/if}
</section>
