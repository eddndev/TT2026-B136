<script>
  import HearingReference from './HearingReference.svelte';
  import { hearingResultTimeLabel } from '../lib/hearing-result-time.mjs';
  import { resultOccurrence, resultExtent, resultSource } from '../lib/hearing-result-errors.mjs';
  import { profileLabel } from '../lib/typed-participant-fields.mjs';
  export let values,
    attendees = [],
    support = null;
  const reference = (row) =>
    row.subject
      ? {
          ...row,
          subject: {
            ...row.subject,
            values_digest: row.subject_digest || row.subject.values_digest,
          },
        }
      : row;
</script>

<dl class="case-values">
  <div>
    <dt>Ocurrencia declarada</dt>
    <dd>{resultOccurrence[values.occurrence]}</dd>
  </div>
  <div>
    <dt>Alcance del acto</dt>
    <dd>{resultExtent[values.extent]}</dd>
  </div>
  <div class="case-value-wide">
    <dt>Tiempo del hecho informado</dt>
    <dd>{hearingResultTimeLabel(values.event_time)}</dd>
  </div>
  <div class="case-value-wide">
    <dt>Relato del operador</dt>
    <dd class="case-multiline">{values.summary}</dd>
  </div>
  <div>
    <dt>Procedencia com&#250;n</dt>
    <dd>{resultSource[values.provenance.kind]}</dd>
  </div>
  {#if values.provenance.reference}<div class="case-value-wide">
      <dt>Localizador declarado</dt>
      <dd>{values.provenance.reference}</dd>
    </div>{/if}
</dl>
{#if values.provenance.support}<div class="case-comparison">
    <p>
      {support?.name || 'Soporte documental'} / Versi&#243;n {values.provenance.support.version}
    </p>
    <details>
      <summary>Referencia exacta del soporte</summary>
      <p><code>{values.provenance.support.document_id}</code></p>
      <code>{values.provenance.support.digest}</code>
    </details>
  </div>{/if}
<h3>Comparecencias informadas</h3>
{#if !values.attendees.length}<p class="hint">Comparecencias no registradas.</p>{/if}
{#each values.attendees as item (`${item.participant_id}:${item.revision}`)}
  {@const row = attendees.find(
    (person) => person.id === item.participant_id && person.revision === item.revision,
  )}
  <div class="hearing-participant">
    <div>
      <strong>{row?.display_name || 'Ficha seleccionada'}</strong>
      {#if row}<p>
          {row.profile === 'typed' || row.kind
            ? profileLabel(row.procedural_role)
            : row.procedural_role} / {row.directory_status === 'archived' ? 'Archivada' : 'Activa'} en
          esta revisi&#243;n
        </p>{/if}
      <p>Calidad en esta sesi&#243;n: {item.capacity}</p>
      {#if item.observation}<p class="case-multiline">{item.observation}</p>{/if}
      <HearingReference
        row={reference(row || { id: item.participant_id, revision: item.revision })}
      />
    </div>
  </div>
{/each}
<p class="hint">Lista declarada; no completa ausencias ni acredita notificaciones.</p>
<h3>Acuerdos declarados</h3>
{#if !values.agreements.length}<p class="hint">Acuerdos no registrados.</p>{/if}
{#each values.agreements as agreement, index (agreement.id)}<div class="hearing-result-agreement">
    <h4>Acuerdo {index + 1}</h4>
    <p class="case-multiline">{agreement.text}</p>
    <details><summary>Identificador del acuerdo</summary><code>{agreement.id}</code></details>
  </div>{/each}
<p class="hint">
  La procedencia corresponde al relato completo. El registro no calcula plazos ni afirma unanimidad
  o firmeza.
</p>
