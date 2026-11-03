<script>
  // "Crear contrato" for the current version of an unsealed PDF: creates the
  // contract and then asks for the signatures of the Client and the firm.
  // The server chooses the contract identifier, so an uncertain creation is
  // reconciled through the receipt of its operation.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { contractable } from '../lib/contract-presentation.mjs';
  import { contractTitle } from '../lib/contract-request-draft.mjs';
  import ContractRequestForm from './ContractRequestForm.svelte';
  export let api, user, document;
  export let disabled = false,
    onupdate = () => {};
  const available = typeof api.contracts === 'function';
  let open = false,
    title = '',
    busy = false,
    error = '',
    notice = '',
    operationId = '',
    uncertain = false,
    created = null,
    firmSigners = [],
    contracts = null,
    scoped = null;
  function start() {
    contracts = api.contracts();
    scoped = contracts.caseContracts(document.case_id);
    title = document.name.replace(/\.pdf$/i, '');
    operationId = crypto.randomUUID();
    error = notice = '';
    open = true;
  }
  async function accepted(receipt) {
    const list = await scoped.list();
    firmSigners = list.firm_signers;
    created = { contractId: receipt.contract_id, revision: receipt.event.revision };
    uncertain = false;
    onupdate({ ...document, contract_state: 'borrador' });
  }
  async function create(event) {
    event.preventDefault();
    if (busy) return;
    let value;
    try {
      value = contractTitle(title);
    } catch (failure) {
      error = failure.message;
      return;
    }
    busy = true;
    error = '';
    try {
      await accepted(await scoped.create({ operationId, documentId: document.id, title: value }));
    } catch (failure) {
      error = failure.message;
      uncertain = !failure.status || failure.status >= 500;
    } finally {
      busy = false;
    }
  }
  async function reconcile() {
    busy = true;
    error = '';
    try {
      await accepted(await scoped.operation(operationId));
    } catch (failure) {
      if (failure.status === 404) {
        uncertain = false;
        error = 'El contrato no se cre\u00f3. Puedes crearlo otra vez.';
      } else error = failure.message;
    } finally {
      busy = false;
    }
  }
  function requested() {
    notice = 'Firmas solicitadas. Consulta su avance en Contratos del expediente.';
    created = null;
    open = false;
    onupdate({ ...document, contract_state: 'en_firma' });
  }
</script>

{#if available && contractable(document, user.role) && !open}
  <button class="secondary" {disabled} onclick={start}>Crear contrato</button>
{/if}
{#if notice}<p class="notice success" role="status">{notice}</p>{/if}
{#if open}<section class="card stack contract-create" aria-label="Crear contrato">
    {#if !created}<form class="stack" onsubmit={create}>
        <h3>Crear contrato sobre la versi&#243;n {document.version}</h3>
        <p class="hint">
          El Cliente y el despacho firmar&#225;n esta versi&#243;n exacta. Despu&#233;s no
          admitir&#225; sello ni firma individual.
        </p>
        <label
          >T&#237;tulo del contrato
          <input required maxlength="160" bind:value={title} disabled={busy || uncertain} />
        </label>
        {#if uncertain}<button type="button" class="secondary" disabled={busy} onclick={reconcile}
            >Consultar recibo del contrato</button
          >{/if}
        <div class="action-row">
          <button class="primary" type="submit" disabled={busy || uncertain}>Crear contrato</button>
          <button type="button" class="secondary" disabled={busy} onclick={() => (open = false)}
            >Cancelar</button
          >
        </div>
      </form>
    {:else}
      <p class="notice success" role="status">Contrato creado en borrador.</p>
      <ContractRequestForm
        {scoped}
        requests={contracts.requests}
        contractId={created.contractId}
        expectedRevision={created.revision}
        version={document.version}
        {firmSigners}
        onrequested={requested}
      />
    {/if}
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  </section>{/if}
