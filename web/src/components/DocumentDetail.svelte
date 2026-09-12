<script>
  import Icon from './Icon.svelte';
  import { can, download } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  let report = document.report || null;
  let busy = '';
  let error = '';
  let message = '';
  let confirmSeal = false;
  const labels = {
    integrity: 'Integridad del archivo',
    signature: 'Firma digital',
    certificate: 'Certificado y revocacion',
    timestamp: 'Sello de tiempo',
  };
  const statusLabels = { passed: 'Correcto', failed: 'Fallo', skipped: 'Sin evaluar' };
  async function run(action) {
    busy = action;
    error = '';
    message = '';
    try {
      if (action === 'seal') {
        const result = await api.seal(document.id);
        report = null;
        onupdate(result);
        message = 'Documento sellado correctamente.';
        confirmSeal = false;
      } else if (action === 'verify') {
        report = null;
        onupdate({ ...document, report: null });
        report = await api.verify(document.id);
        onupdate({ ...document, report });
      } else {
        const result = await api.evidence(document.id);
        download(result.blob, `evidencia-${document.id}.zip`);
        message = 'Evidencia descargada.';
      }
    } catch (failure) {
      error = failure.message;
    } finally {
      busy = '';
    }
  }
</script>

<section class="detail-panel" aria-labelledby="document-title">
  <div class="section-heading">
    <span class="eyebrow">DOCUMENTO ABIERTO</span><span
      class:success={document.sealed}
      class="badge"
      >{document.sealed ? 'Sellado' : document.version ? 'Sin sellar' : 'Por identificador'}</span
    >
  </div>
  <div class="detail-title">
    <span class="file-icon"><Icon name="file" size={26} /></span>
    <div>
      <h2 id="document-title">{document.name}</h2>
      <p>
        {document.version ? `Version ${document.version}` : 'Referencia de un documento existente'}
      </p>
    </div>
  </div>
  <label>Identificador del documento<input readonly value={document.id} /></label>
  <p class="hint">Conserva este identificador para volver a abrir el documento en otra sesion.</p>
  {#if document.digest || report?.document_digest}<div class="digest">
      <span class="eyebrow">RESUMEN SHA-256</span><code
        >{report?.document_digest || document.digest}</code
      >
    </div>{/if}
  <div class="action-row">
    <button class="primary" disabled={!!busy} onclick={() => run('verify')}
      ><Icon name="shield" size={17} />{busy === 'verify'
        ? 'Verificando...'
        : 'Verificar integridad'}</button
    >
    {#if can(user.role, 'seal') && !document.sealed}<button
        class="secondary"
        disabled={!!busy}
        onclick={() => (confirmSeal = true)}>Sellar documento</button
      >{/if}
    <button class="secondary" disabled={!!busy} onclick={() => run('evidence')}
      ><Icon name="download" size={17} />{busy === 'evidence'
        ? 'Descargando...'
        : 'Descargar evidencia'}</button
    >
  </div>
  {#if confirmSeal}<div class="notice stack">
      <strong>Firmar y sellar este documento?</strong>
      <p>
        Se registrara la operacion con tu identidad y se generara evidencia con la autoridad de
        sellado local. Esta accion no se puede deshacer desde la interfaz.
      </p>
      <div class="action-row">
        <button class="primary" disabled={!!busy} onclick={() => run('seal')}
          >{busy === 'seal' ? 'Sellando...' : 'Confirmar sellado'}</button
        ><button class="secondary" disabled={!!busy} onclick={() => (confirmSeal = false)}
          >Cancelar</button
        >
      </div>
    </div>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if message}<p class="notice success" role="status">{message}</p>{/if}
  {#if report}
    <div class="verification">
      <div class="section-heading">
        <h3>Resultado de verificacion</h3>
        <span
          class="badge"
          class:success={report.verdict === 'valid'}
          class:danger={report.verdict !== 'valid'}
          >{report.verdict === 'valid' ? 'Verificacion valida' : 'Verificacion no valida'}</span
        >
      </div>
      {#each Object.entries(labels) as [key, label]}<div class="check-row">
          <Icon name={report[key].status === 'passed' ? 'check' : 'shield'} size={18} />
          <div>
            <strong>{label}</strong>
            <p>{report[key].detail}</p>
          </div>
          <span
            class="badge"
            class:success={report[key].status === 'passed'}
            class:danger={report[key].status === 'failed'}
            >{statusLabels[report[key].status] || report[key].status}</span
          >
        </div>{/each}
    </div>
  {/if}
  <p class="hint local-tsa">
    El sellado local es evidencia tecnica del prototipo; no es una constancia NOM-151 emitida por un
    PSC autorizado.
  </p>
</section>
