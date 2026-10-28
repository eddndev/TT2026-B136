<script>
  // The result of one evidence validation: the overall result, each
  // component, the instants with their sources, the roots and rules applied,
  // the comparison with the server, the informational current status and
  // the JSON download of the run. Rationale:
  // docs/adr/0085-in-browser-evidence-validation.md.
  import Icon from '@qadra/ui/Icon.svelte';
  import { download } from '../lib/documents.mjs';
  import { HISTORICAL_NOTE, NOT_QADRA_EXPORT } from '../lib/evidence/messages.mjs';
  import {
    PURPOSE_LABELS,
    appliedRoots,
    componentView,
    currentStatusView,
    instantView,
    isNotQadraExport,
    overallLabel,
    profileLabel,
    serverComparison,
    signatureNote,
  } from '../lib/evidence/presentation.mjs';
  import { exportReport, reportFilename } from '../lib/evidence/report-export.mjs';
  export let report;
  export let policy;
  export let route = null;
  export let document;
  export let file;
  export let pinned = [];
  $: overall = overallLabel(report);
  $: foreign = isNotQadraExport(report);
  $: note = signatureNote(report);
  $: components = (report.components || []).map(componentView);
  $: instants = (report.instants || []).map(instantView);
  $: roots = appliedRoots(report, pinned);
  $: comparison = serverComparison(report);
  $: current = currentStatusView(report);
  const purposes = (root) => root.purposes.map((purpose) => PURPOSE_LABELS[purpose]).join(', ');

  function save() {
    const value = exportReport({
      report,
      engine: report.engine,
      policy,
      pinned,
      route,
      document,
      file,
      generatedAt: new Date(),
    });
    const blob = new Blob([`${JSON.stringify(value, null, 2)}\n`], { type: 'application/json' });
    download(blob, reportFilename(document));
  }
</script>

<section class="evidence-result" aria-labelledby="evidence-result-title">
  <div class="evidence-result-heading">
    <div>
      <h4 id="evidence-result-title">Resultado de la validaci&#243;n</h4>
      <p>{overall.summary}</p>
    </div>
    <span
      class="badge evidence-result-overall"
      class:success={overall.tone === 'success'}
      class:warning={overall.tone === 'warning'}
      class:danger={overall.tone === 'danger'}>{overall.label}</span
    >
  </div>
  {#if foreign}<p class="notice error evidence-result-foreign">{NOT_QADRA_EXPORT}</p>{/if}
  {#if note}<p class="notice evidence-result-signature">{note}</p>{/if}
  <p class="evidence-result-meta">
    Archivo: <strong>{file.name}</strong>. Tipo de paquete: {profileLabel(report.profile)}.
  </p>
  <ul class="evidence-result-components">
    {#each components as item}<li>
        <div class="evidence-result-row">
          <strong>{item.title}</strong>
          <span
            class="badge"
            class:success={item.tone === 'success'}
            class:warning={item.tone === 'warning'}
            class:danger={item.tone === 'danger'}>{item.status}</span
          >
        </div>
        {#if item.profile}<p>Perfil de los certificados: {item.profile}.</p>{/if}
        {#if item.trust}<p>Ra&#237;z de confianza: {item.trust}.</p>{/if}
        {#each item.messages.filter((text) => !foreign || text !== NOT_QADRA_EXPORT) as text}<p
            class="evidence-result-message"
          >
            {text}
          </p>{/each}
      </li>{/each}
  </ul>
  {#if instants.length}<h5>Fechas usadas</h5>
    <ul class="evidence-result-instants">
      {#each instants as instant}<li>
          <strong>{instant.label}</strong><span>{instant.when}</span><small>{instant.source}</small>
        </li>{/each}
    </ul>{/if}
  <h5>Reglas aplicadas</h5>
  <ul class="evidence-result-anchors">
    {#each roots as root}<li>Ra&#237;z fija: {root.label} ({purposes(root)})</li>{:else}<li>
        Ninguna ra&#237;z de la lista fija de Qadra se pudo aplicar.
      </li>{/each}
    <li>
      Perfil estricto de certificados y listas de revocaci&#243;n de esta versi&#243;n de Qadra.
    </li>
    <li>Pol&#237;tica de sello de tiempo admitida: {policy.admitted_policies.join(', ')}</li>
    <li>{HISTORICAL_NOTE}</li>
  </ul>
  <h5>Comparaci&#243;n con el servidor</h5>
  <ul class="evidence-result-notes">
    {#each comparison as text}<li>{text}</li>{/each}
  </ul>
  {#if current.length}<h5>Estado actual (informativo)</h5>
    <p class="evidence-result-current">
      Se calcula con el reloj de este equipo y no cambia el resultado.
    </p>
    <ul class="evidence-result-notes">
      {#each current as row}<li class:warning={row.tone !== 'success'}>
          {#if row.subject}<strong>{row.subject}:</strong>{/if}
          {row.message}
        </li>{/each}
    </ul>{/if}
  <details class="technical-detail">
    <summary>Ver detalles t&#233;cnicos de la validaci&#243;n</summary>
    <p>
      Motor {report.engine?.name}
      {report.engine?.version}. Resumen SHA-256 del archivo:
      <code>{file.sha256}</code>
    </p>
    {#each report.digests || [] as digest}<p>{digest.name}: <code>{digest.sha256}</code></p>{/each}
    {#each report.components || [] as component}{#each component.findings as finding}<p>
          <code>{finding.code}</code>
          {finding.detail}
        </p>{/each}{/each}
  </details>
  <div class="evidence-result-actions">
    <button class="secondary" onclick={save}
      ><Icon name="download" size={17} />Descargar informe JSON</button
    >
    <p class="hint">El informe describe esta revisi&#243;n; no es una firma ni un sello nuevo.</p>
  </div>
</section>
