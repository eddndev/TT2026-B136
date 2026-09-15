<script>
  import Icon from './Icon.svelte';
  import { can, download } from '../lib/documents.mjs';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  let report = document.report || null;
  let activeTab = report ? 'verification' : 'summary';
  let busy = '';
  let error = '';
  let message = '';
  let copyMessage = '';
  let confirmSeal = false;
  $: report = document.report || null;
  const tabs = [
    { id: 'summary', label: 'Resumen', icon: 'file' },
    { id: 'verification', label: 'Verificaci\u00f3n', icon: 'shield' },
    { id: 'evidence', label: 'Evidencia', icon: 'download' },
  ];
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
  function moveTab(event, index) {
    const key = event.key;
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(key)) return;
    event.preventDefault();
    const next =
      key === 'Home'
        ? 0
        : key === 'End'
          ? tabs.length - 1
          : (index + (key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length;
    activeTab = tabs[next].id;
    event.currentTarget.parentElement.querySelectorAll('[role="tab"]')[next].focus();
  }
  async function copyIdentifier() {
    copyMessage = '';
    try {
      await navigator.clipboard.writeText(document.id);
      copyMessage = 'Identificador copiado.';
    } catch {
      copyMessage = 'No se pudo copiar. Selecciona el identificador y c\u00f3pialo manualmente.';
    }
  }
  async function run(action) {
    if (busy || (document.sealed === false && action !== 'seal')) return;
    busy = action;
    error = '';
    message = '';
    try {
      if (action === 'seal') {
        const result = await api.seal(document.id);
        report = null;
        onupdate({ ...document, ...result, report: null });
        message = 'Documento sellado correctamente.';
        confirmSeal = false;
      } else if (action === 'verify') {
        activeTab = 'verification';
        report = null;
        onupdate({ ...document, report: null });
        report = await api.verify(document.id);
        onupdate({
          ...document,
          sealed: true,
          digest: report.document_digest || document.digest,
          report,
        });
      } else {
        const result = await api.evidence(document.id);
        download(result.blob, `evidencia-${document.id}.zip`);
        message = 'Evidencia descargada.';
        activeTab = 'evidence';
      }
    } catch (failure) {
      error = failure.message;
      if (failure.status === 409 && failure.code === 'document_not_sealed') {
        report = null;
        onupdate({ ...document, sealed: false, report: null });
      }
    } finally {
      busy = '';
    }
  }
</script>

<section class="detail-panel" aria-labelledby="document-title">
  <div class="section-heading">
    <span class="eyebrow">FICHA DOCUMENTAL</span>
    <span
      class="badge"
      class:success={document.sealed === true}
      class:warning={document.sealed === false}
      >{document.sealed === true
        ? 'Sellado'
        : document.sealed === false
          ? 'Pendiente de sello'
          : 'Estado por consultar'}</span
    >
  </div>
  <div class="detail-title">
    <span class="file-icon"><Icon name="file" size={26} /></span>
    <div>
      <h2 id="document-title">{document.name}</h2>
      <p>
        {document.version
          ? `Versi\u00f3n ${document.version}`
          : 'Documento recuperado por identificador'}
      </p>
    </div>
  </div>
  <div class="document-actions" aria-label="Acciones del documento">
    {#if can(user.role, 'seal') && document.sealed !== true}<button
        class="primary"
        disabled={!!busy}
        onclick={() => (confirmSeal = true)}><Icon name="lock" size={17} />Sellar documento</button
      >{/if}
    <button
      class="secondary"
      disabled={!!busy || document.sealed === false}
      onclick={() => run('verify')}
      ><Icon name="shield" size={17} />{busy === 'verify'
        ? 'Verificando...'
        : 'Verificar integridad'}</button
    >
    <button
      class="secondary"
      disabled={!!busy || document.sealed === false}
      onclick={() => run('evidence')}
      ><Icon name="download" size={17} />{busy === 'evidence'
        ? 'Descargando...'
        : 'Descargar evidencia'}</button
    >
  </div>
  {#if document.sealed === false}<p class="detail-next-step">
      <Icon name="info" size={16} /><span
        >{can(user.role, 'seal')
          ? 'Sella el documento para habilitar su verificaci\u00f3n y descarga de evidencia.'
          : 'Solicita al administrador o a un litigante que selle el documento para continuar.'}</span
      >
    </p>{/if}
  {#if confirmSeal}<div class="notice stack seal-confirmation">
      <strong>&#191;Firmar y sellar este documento?</strong>
      <p>
        Se registrar&#225; la operaci&#243;n con tu identidad y se generar&#225; evidencia con la
        autoridad de sellado local. Esta acci&#243;n no se puede deshacer desde la interfaz.
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
  <div class="detail-tabs" role="tablist" aria-label="Informaci&#243;n del documento">
    {#each tabs as tab, index}<button
        id={`document-tab-${tab.id}`}
        role="tab"
        aria-selected={activeTab === tab.id}
        aria-controls={`document-panel-${tab.id}`}
        tabindex={activeTab === tab.id ? 0 : -1}
        class:active={activeTab === tab.id}
        onclick={() => (activeTab = tab.id)}
        onkeydown={(event) => moveTab(event, index)}
        ><Icon name={tab.icon} size={17} />{tab.label}</button
      >{/each}
  </div>
  <div
    class="detail-tab-content"
    role="tabpanel"
    id={`document-panel-${activeTab}`}
    aria-labelledby={`document-tab-${activeTab}`}
    tabindex="0"
  >
    {#if activeTab === 'summary'}
      <div class="detail-summary-heading">
        <h3>El recorrido de tu documento</h3>
        <p>Consulta su estado antes de continuar con el siguiente paso.</p>
      </div>
      <ol class="document-progress" aria-label="Estado del documento">
        <li class="complete">
          <span><Icon name="check" size={16} /></span>
          <div><strong>Archivo abierto</strong><small>Listo para trabajar</small></div>
        </li>
        <li class:complete={document.sealed === true}>
          <span
            >{#if document.sealed === true}<Icon name="check" size={16} />{:else}2{/if}</span
          >
          <div>
            <strong>Sellado</strong><small
              >{document.sealed === true
                ? 'Sello registrado'
                : document.sealed === false
                  ? 'Pendiente'
                  : 'Por consultar'}</small
            >
          </div>
        </li>
        <li class:complete={report?.verdict === 'valid'}>
          <span
            >{#if report?.verdict === 'valid'}<Icon name="check" size={16} />{:else}3{/if}</span
          >
          <div>
            <strong>Verificaci&#243;n</strong><small
              >{report
                ? report.verdict === 'valid'
                  ? 'Comprobada'
                  : 'Requiere revisi\u00f3n'
                : 'Sin resultado vigente'}</small
            >
          </div>
        </li>
      </ol>
      <div class="identifier-card">
        <label for="document-identifier">Identificador del documento</label>
        <div class="identifier-control">
          <input id="document-identifier" readonly value={document.id} /><button
            class="secondary"
            onclick={copyIdentifier}
            aria-label="Copiar identificador"><Icon name="copy" size={16} />Copiar</button
          >
        </div>
        <p class="hint">
          Puedes usarlo para abrir este archivo dentro del expediente o compartir su referencia con
          integrantes que tengan acceso.
        </p>
        <span class="copy-feedback" role="status" aria-live="polite">{copyMessage}</span>
      </div>
      {#if document.digest || report?.document_digest}<details class="technical-detail">
          <summary>Consultar resumen SHA-256</summary>
          <p>
            Esta huella identifica el contenido del archivo. Se utiliza al comprobar su integridad.
          </p>
          <code>{report?.document_digest || document.digest}</code>
        </details>{/if}
    {:else if activeTab === 'verification'}
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
          <summary>Ver detalles t&#233;cnicos de la verificaci&#243;n</summary
          >{#each checks as check}<div class="technical-check">
              <strong>{check.label}</strong>
              <p>{report[check.key]?.detail || 'Sin detalle disponible.'}</p>
            </div>{/each}{#if report.document_digest}<span class="eyebrow">RESUMEN SHA-256</span
            ><code>{report.document_digest}</code>{/if}
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
    {:else}
      <div class="evidence-overview">
        <span class="tile-icon"><Icon name="download" size={27} /></span>
        <div>
          <h3>Tu paquete de evidencia</h3>
          <p>
            Descarga un ZIP para conservar el archivo y los elementos necesarios para comprobar su
            evidencia fuera de Qadra.
          </p>
        </div>
      </div>
      <ul class="evidence-contents">
        <li><Icon name="file" size={18} /><span>Documento y firma digital</span></li>
        <li><Icon name="shield" size={18} /><span>Sello de tiempo y certificados</span></li>
        <li>
          <Icon name="check" size={18} /><span>Instrucciones para verificar la evidencia</span>
        </li>
      </ul>
      <p class="evidence-guidance">
        {document.sealed === false
          ? 'La descarga estar\u00e1 disponible despu\u00e9s de sellar el documento.'
          : 'Usa Descargar evidencia en las acciones de esta ficha. Conserva el ZIP completo para su verificaci\u00f3n.'}
      </p>
    {/if}
  </div>
  <p class="hint local-tsa">
    El sellado local es evidencia t&#233;cnica del prototipo; no es una constancia NOM-151 emitida
    por un PSC autorizado.
  </p>
</section>
