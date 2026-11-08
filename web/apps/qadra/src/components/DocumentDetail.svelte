<script>
  import Button from './Button.svelte';
  import { onDestroy } from 'svelte';
  import { caseState } from '../lib/case-state.mjs';
  const administration = caseState();
  import Icon from '@qadra/ui/Icon.svelte';
  import DocumentContentAction from './DocumentContentAction.svelte';
  import DocumentSigningActions from './DocumentSigningActions.svelte';
  import DocumentSealAction from './DocumentSealAction.svelte';
  import DocumentVerificationReport from './DocumentVerificationReport.svelte';
  import EvidenceValidation from './EvidenceValidation.svelte';
  import DocumentContractAction from './DocumentContractAction.svelte';
  import SectionTabs from './SectionTabs.svelte';
  import { can, download } from '../lib/documents.mjs';
  import { contractDocumentStatus } from '../lib/contract-presentation.mjs';
  export let api;
  export let user;
  export let document;
  export let onupdate;
  export let disabled = false;
  export let ondenied = () => {};
  let alive = true;
  onDestroy(() => {
    alive = false;
    busy = '';
  });
  let report = document.report || null;
  let activeTab = report ? 'verification' : 'summary';
  export let busy = '';
  let error = '';
  let message = '';
  let copyMessage = '';
  let sealAction;
  let sealUncertain = false,
    personalActive = false;
  $: report = document.report || null;
  // A version under contract never reads as sealed or pending seal.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  $: contract = contractDocumentStatus(document);
  const tabs = [
    { id: 'summary', label: 'Resumen', icon: 'document' },
    { id: 'verification', label: 'Verificaci\u00f3n', icon: 'shield' },
    { id: 'evidence', label: 'Evidencia', icon: 'download' },
  ];
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
    if (busy || disabled || document.sealed === false) return;
    busy = action;
    error = '';
    message = '';
    try {
      if (action === 'verify') {
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
        download(result.blob, `evidencia-${document.id}-v${document.version}.zip`);
        message = 'Evidencia descargada.';
        activeTab = 'evidence';
      }
    } catch (failure) {
      if (!alive) return;
      error = failure.message;
      if ([403, 404].includes(failure.status)) ondenied(failure);
      if (failure.status === 409 && failure.code === 'document_not_sealed') {
        report = null;
        onupdate({ ...document, sealed: false, report: null });
      }
    } finally {
      if (alive) busy = '';
    }
  }
</script>

<section class="detail-panel" aria-labelledby="document-title">
  <div class="section-heading">
    <span class="eyebrow">FICHA DOCUMENTAL</span>
    <span
      class="badge"
      class:success={document.sealed === true}
      class:warning={document.sealed === false && !contract}
      class:info={!!contract}
      >{contract
        ? contract.label
        : document.sealed === true
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
    <DocumentContentAction
      {api}
      {document}
      {disabled}
      {ondenied}
      bind:busy
      bind:error
      bind:message
    />
    {#if can(user.role, 'seal') && document.sealed !== true && !contract}<Button
        variant="primary"
        disabled={disabled || !!busy || sealUncertain || personalActive || $administration.closed}
        onclick={() => sealAction.open()}><Icon name="lock" size={17} />Sellar documento</Button
      >{/if}
    <Button
      variant="secondary"
      disabled={disabled || !!busy || document.sealed === false}
      onclick={() => run('verify')}
      ><Icon name="shield" size={17} />{busy === 'verify'
        ? 'Verificando...'
        : 'Verificar integridad'}</Button
    >
    <Button
      variant="secondary"
      disabled={disabled || !!busy || document.sealed === false}
      onclick={() => run('evidence')}
      ><Icon name="download" size={17} />{busy === 'evidence'
        ? 'Descargando...'
        : 'Descargar evidencia'}</Button
    >
  </div>
  {#if disabled && !busy}<p class="hint" role="status">
      Actualizando los datos del documento...
    </p>{/if}
  {#if contract}<p class="detail-next-step">
      <Icon name="info" size={16} /><span
        >Esta versi&#243;n est&#225; bajo contrato: no admite sello ni firma individual. Consulta su
        estado y su evidencia en Contratos del expediente.</span
      >
    </p>
  {:else if document.sealed === false}<p class="detail-next-step">
      <Icon name="info" size={16} /><span
        >{can(user.role, 'seal')
          ? 'Sella el documento para habilitar su verificaci\u00f3n y descarga de evidencia.'
          : 'Solicita al administrador o a un litigante que selle el documento para verificarlo o descargar su evidencia.'}</span
      >
    </p>{/if}
  <DocumentSealAction
    bind:this={sealAction}
    {api}
    {document}
    disabled={disabled || personalActive}
    {onupdate}
    {ondenied}
    bind:busy
    bind:error
    bind:message
    bind:uncertain={sealUncertain}
  />
  <DocumentSigningActions
    {api}
    {user}
    {document}
    {onupdate}
    {ondenied}
    {disabled}
    bind:busy
    bind:active={personalActive}
  />
  <DocumentContractAction {api} {user} {document} {disabled} {onupdate} />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if message}<p class="notice success" role="status">{message}</p>{/if}
  <SectionTabs
    items={tabs}
    current={activeTab}
    label="Informaci&#243;n del documento"
    onselect={(id) => (activeTab = id)}
    mode="tabs"
    idPrefix="document"
  />

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
                : contract
                  ? 'Bajo contrato'
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
          <input id="document-identifier" readonly value={document.id} /><Button
            variant="secondary"
            onclick={copyIdentifier}
            aria-label="Copiar identificador"><Icon name="copy" size={16} />Copiar</Button
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
      <DocumentVerificationReport {document} {report} {busy} />
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
        {contract
          ? 'La evidencia de esta versi\u00f3n es el paquete del contrato; desc\u00e1rgalo desde Contratos.'
          : document.sealed === false
            ? 'La descarga estar\u00e1 disponible despu\u00e9s de sellar el documento.'
            : 'Usa Descargar evidencia en las acciones de esta ficha. Conserva el ZIP completo para su verificaci\u00f3n.'}
      </p>
      <EvidenceValidation {api} {document} />
    {/if}
  </div>
  <p class="hint local-tsa">
    El sellado local es evidencia t&#233;cnica del prototipo; no es una constancia NOM-151 emitida
    por un PSC autorizado.
  </p>
</section>
