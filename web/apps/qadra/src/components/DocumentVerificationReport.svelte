<script>
  import Icon from '@qadra/ui/Icon.svelte';
  export let document;
  export let report;
  export let busy = '';
  const checks = [
    {
      key: 'integrity',
      label: 'Integridad del archivo',
      passed: 'El contenido coincide con el resumen registrado.',
    },
    {
      key: 'signature',
      label: 'Firma digital',
      passed: 'La firma del documento pas\u00f3 la comprobaci\u00f3n.',
    },
    {
      key: 'certificate',
      label: 'Certificado y revocaci\u00f3n',
      passed: 'El certificado y su estado de revocaci\u00f3n fueron comprobados.',
    },
    {
      key: 'timestamp',
      label: 'Sello de tiempo',
      passed: 'El sello de tiempo local pas\u00f3 la comprobaci\u00f3n.',
    },
  ];
  const statusLabels = { passed: 'Correcto', failed: 'Revisar', skipped: 'Sin evaluar' };
</script>

{#if report}
  <div class="verification-heading">
    <div>
      <h3>Resultado de verificaci&#243;n</h3>
      <p>Comprobaciones realizadas sobre la evidencia del documento.</p>
    </div>
    <span
      class="badge"
      class:success={report.verdict === 'valid'}
      class:danger={report.verdict !== 'valid'}
      >{report.verdict === 'valid'
        ? 'Verificaci\u00f3n v\u00e1lida'
        : 'Verificaci\u00f3n no v\u00e1lida'}</span
    >
  </div>
  <div class="verification-checks">
    {#each checks as check}<div class="check-row">
        <span
          class="check-icon"
          class:passed={report[check.key]?.status === 'passed'}
          class:failed={report[check.key]?.status === 'failed'}
          ><Icon
            name={report[check.key]?.status === 'passed' ? 'check' : 'shield'}
            size={18}
          /></span
        >
        <div>
          <strong>{check.label}</strong>
          <p>
            {report[check.key]?.status === 'passed'
              ? check.passed
              : report[check.key]?.status === 'failed'
                ? 'La comprobaci\u00f3n detect\u00f3 un problema. Revisa el detalle t\u00e9cnico antes de utilizar esta evidencia.'
                : 'Esta comprobaci\u00f3n no tiene un resultado disponible.'}
          </p>
        </div>
        <span
          class="badge"
          class:success={report[check.key]?.status === 'passed'}
          class:danger={report[check.key]?.status === 'failed'}
          >{statusLabels[report[check.key]?.status] || 'Sin evaluar'}</span
        >
      </div>{/each}
  </div>
  <details class="technical-detail">
    <summary>Ver detalles t&#233;cnicos de la verificaci&#243;n</summary>{#each checks as check}<div
        class="technical-check"
      >
        <strong>{check.label}</strong>
        <p>{report[check.key]?.detail || 'Sin detalle disponible.'}</p>
      </div>{/each}{#if report.document_digest}<span class="eyebrow">RESUMEN SHA-256</span><code
        >{report.document_digest}</code
      >{/if}
  </details>
{:else}<div class="detail-empty">
    <span class="tile-icon"><Icon name="shield" size={26} /></span>
    <h3>
      {busy === 'verify' ? 'Comprobando el documento' : 'Sin verificaci\u00f3n vigente'}
    </h3>
    <p>
      {busy === 'verify'
        ? 'Espera mientras se revisan la integridad, la firma y el sello de tiempo.'
        : document.sealed === false
          ? 'Primero se necesita el sello. Despu\u00e9s podr\u00e1s comprobar la integridad del archivo.'
          : 'Usa Verificar integridad para obtener un resultado actualizado de este documento.'}
    </p>
  </div>{/if}
