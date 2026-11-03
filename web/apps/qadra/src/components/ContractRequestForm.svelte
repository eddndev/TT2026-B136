<script>
  // Requests the signatures of one contract: the Client by the exact e-mail
  // of its account (the server resolves it), the firm signer, the exact
  // version, the validity and the fingerprint sheet reference. A refusal
  // lists the readiness blockers and creates nothing, so the validity does
  // not run while a party completes its onboarding.
  // Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { blockerMessages } from '../lib/contract-presentation.mjs';
  import { DEFAULT_TTL_DAYS, contractRequestValues } from '../lib/contract-request-draft.mjs';
  export let scoped, requests, contractId, expectedRevision, version, firmSigners;
  export let onrequested = () => {};
  let clientEmail = '',
    firmSignerId = '',
    ttlDays = DEFAULT_TTL_DAYS,
    ceremonyReference = '',
    busy = false,
    error = '',
    blockers = [],
    pending = null;
  async function submit(event) {
    event.preventDefault();
    if (busy) return;
    error = '';
    blockers = [];
    let values;
    try {
      values = contractRequestValues({
        clientEmail,
        firmSignerId,
        ttlDays,
        version,
        ceremonyReference,
      });
    } catch (failure) {
      error = failure.message;
      return;
    }
    const attempt = {
      ...values,
      requestId: crypto.randomUUID(),
      operationId: crypto.randomUUID(),
      expectedRevision,
    };
    busy = true;
    try {
      const receipt = await scoped.request(contractId, attempt);
      pending = null;
      onrequested(receipt);
    } catch (failure) {
      error = failure.message;
      blockers = blockerMessages(failure.blockers);
      if (!failure.status || failure.status >= 500) pending = attempt;
    } finally {
      busy = false;
    }
  }
  async function reconcile() {
    if (busy || !pending) return;
    busy = true;
    error = '';
    try {
      const receipt = await requests.operation(pending.requestId, pending.operationId);
      pending = null;
      onrequested(receipt);
    } catch (failure) {
      error =
        failure.status === 404
          ? 'La solicitud no se registr\u00f3. Puedes enviarla otra vez.'
          : failure.message;
      if (failure.status === 404) pending = null;
    } finally {
      busy = false;
    }
  }
</script>

<form class="card stack contract-request" aria-label="Solicitar firmas" onsubmit={submit}>
  <h3>Solicitar firmas</h3>
  <p class="hint">
    Versi&#243;n {version}. Firma primero el Cliente y despu&#233;s quien firma por el despacho.
  </p>
  <label
    >Correo de la cuenta del Cliente
    <input type="email" required maxlength="254" bind:value={clientEmail} disabled={busy} />
  </label>
  <label
    >Firma por el despacho
    <select required bind:value={firmSignerId} disabled={busy}>
      <option value="">Elige una cuenta</option>
      {#each firmSigners as signer}<option value={signer.account_id}
          >{signer.email} ({signer.role === 'owner' ? 'Owner' : 'Litigante'})</option
        >{/each}
    </select>
  </label>
  <label
    >Vigencia en d&#237;as
    <input type="number" min="1" max="30" required bind:value={ttlDays} disabled={busy} />
  </label>
  <label
    >Folio de la hoja de huella firmada
    <input required maxlength="80" bind:value={ceremonyReference} disabled={busy} />
  </label>
  {#if blockers.length}<section class="notice stack" aria-label="Lista de preparacion">
      <h4>Antes de solicitar las firmas</h4>
      <ul>
        {#each blockers as line}<li>{line}</li>{/each}
      </ul>
      <p class="hint">No se cre&#243; la solicitud y su vigencia no ha empezado.</p>
    </section>{/if}
  {#if error && !blockers.length}<p class="notice error" role="alert">{error}</p>{/if}
  {#if pending}<button type="button" class="secondary" disabled={busy} onclick={reconcile}
      >Consultar recibo de la solicitud</button
    >{/if}
  <button class="primary" type="submit" disabled={busy || !!pending}>Solicitar firmas</button>
</form>
