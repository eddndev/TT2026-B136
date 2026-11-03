<script>
  // One contract of a case for its staff: state, timeline of events,
  // signature requests with their downloads, a notice when a later document
  // version exists, cancellation, abandonment and new requests. An assigned
  // Paralegal reads only. Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { onDestroy, onMount } from 'svelte';
  import { download } from '../lib/documents.mjs';
  import {
    contractStateView,
    eventLabel,
    expiryText,
    requestActions,
    requestStatusView,
  } from '../lib/contract-presentation.mjs';
  import ContractCloseAction from './ContractCloseAction.svelte';
  import ContractRequestForm from './ContractRequestForm.svelte';
  export let api, scoped, requests, user, caseId, contractId, firmSigners;
  export let onchanged = () => {};
  const manage = ['owner', 'litigator'].includes(user.role);
  let contract = null,
    currentVersion = null,
    busy = false,
    error = '',
    notice = '',
    alive = true;
  $: state = contract ? contractStateView(contract.state) : null;
  $: open = contract?.requests.find((row) => requestStatusView(row.status).open) ?? null;
  $: superseded = open && currentVersion !== null && open.version < currentVersion;
  async function run(action) {
    if (busy) return;
    busy = true;
    error = '';
    try {
      await action();
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      if (alive) busy = false;
    }
  }
  async function load() {
    await run(async () => {
      const value = await scoped.get(contractId);
      const documents = api.caseDocuments(caseId);
      let version = null;
      try {
        version = (await documents.detail(value.document_id)).version;
      } catch {
        version = null;
      } finally {
        documents.dispose?.();
      }
      if (!alive) return;
      contract = value;
      currentVersion = version;
    });
  }
  async function cancel(row, reason) {
    await requests.close(row.request_id, {
      operationId: crypto.randomUUID(),
      expectedRevision: contract.revision,
      outcome: 'cancel',
      reason,
    });
    notice = 'Solicitud cancelada.';
    await load();
    onchanged();
  }
  async function abandon(reason) {
    await scoped.abandon(contractId, {
      operationId: crypto.randomUUID(),
      expectedRevision: contract.revision,
      reason,
    });
    notice = 'Contrato abandonado.';
    await load();
    onchanged();
  }
  function evidence(row, stage) {
    run(async () => {
      const blob = await requests.evidence(row.request_id, stage);
      download(blob, `contrato-${row.request_id}-${stage === 'partial' ? 'parcial' : 'final'}.zip`);
    });
  }
  async function requested() {
    notice = 'Firmas solicitadas. El Cliente ya puede revisar y firmar.';
    await load();
    onchanged();
  }
  onMount(load);
  onDestroy(() => (alive = false));
</script>

<section class="card stack contract-detail" aria-label="Detalle del contrato" aria-busy={busy}>
  {#if !contract}<p role="status">{busy ? 'Consultando el contrato...' : ''}</p>
  {:else}
    <div class="section-heading">
      <div>
        <span class="eyebrow">CONTRATO</span>
        <h2>{contract.title}</h2>
        <p class="hint">Creado por {contract.created_by_email}</p>
      </div>
      <span class="badge {state.tone}">{state.label}</span>
    </div>
    {#if notice}<p class="notice success" role="status">{notice}</p>{/if}
    {#if superseded}<p class="notice error" role="alert">
        Existe una versi&#243;n posterior del documento (versi&#243;n {currentVersion}). La
        solicitud abierta ya no podr&#225; firmarse: canc&#233;lala y solicita firmas sobre la
        versi&#243;n actual.
      </p>{/if}
    <h3>Solicitudes de firma</h3>
    {#each contract.requests as row (row.request_id)}{@const view = requestStatusView(row.status)}
      {@const actions = requestActions(row, user)}
      <article class="contract-request-row stack">
        <span class="badge {view.tone}">{view.label}</span>
        <p>
          Versi&#243;n {row.version}. Vence: {expiryText(row.expires_at_unix)}. Folio {row.ceremony_reference}.
        </p>
        {#if row.client_signed_at}<p class="hint">
            Firm&#243; el Cliente: {row.client_signed_at}
          </p>{/if}
        {#if row.firm_signed_at}<p class="hint">
            Firm&#243; el despacho: {row.firm_signed_at}
          </p>{/if}
        <div class="action-row">
          {#if actions.partial}<button
              class="secondary"
              disabled={busy}
              onclick={() => evidence(row, 'partial')}>Descargar acuse parcial</button
            >{/if}
          {#if actions.final}<button
              class="secondary"
              disabled={busy}
              onclick={() => evidence(row, 'final')}>Descargar contrato firmado</button
            >{/if}
          {#if manage && actions.cancel}<ContractCloseAction
              outcome="cancel"
              disabled={busy}
              onconfirm={(reason) => cancel(row, reason)}
            />{/if}
        </div>
      </article>
    {:else}<p class="hint">Todav&#237;a no se han solicitado firmas.</p>{/each}
    <h3>L&#237;nea de tiempo</h3>
    <ol class="contract-timeline">
      {#each contract.events as event (event.revision)}<li>
          <strong>{eventLabel(event.kind)}</strong>
          <span class="hint">{event.at} / {event.actor_label ?? 'Sistema'}</span>
          {#if event.reason}<span>Motivo: {event.reason}</span>{/if}
        </li>{/each}
    </ol>
    {#if manage && contract.state === 'borrador' && !open && currentVersion}
      <ContractRequestForm
        {scoped}
        {requests}
        {contractId}
        expectedRevision={contract.revision}
        version={currentVersion}
        {firmSigners}
        onrequested={requested}
      />
      <ContractCloseAction outcome="abandon" disabled={busy} onconfirm={abandon} />
    {/if}
  {/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</section>

<style>
  .contract-detail,
  .contract-request-row {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .contract-request-row {
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }
  .contract-timeline {
    display: grid;
    gap: 8px;
    padding-left: 20px;
  }
  .contract-timeline li {
    display: grid;
    gap: 2px;
  }
</style>
