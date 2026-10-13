<script>
  import { onDestroy } from 'svelte';
  import Icon from '@qadra/ui/Icon.svelte';
  import CaRequest from './CaRequest.svelte';
  import CaCertificates from './CaCertificates.svelte';
  import CaHistory from './CaHistory.svelte';
  import CaRevocation from './CaRevocation.svelte';
  import CaPublication from './CaPublication.svelte';
  import { createCaOperations } from '../lib/ca-operations-model.mjs';
  import { createCaRevocation } from '../lib/ca-revocation-model.mjs';
  import { createCaPublication } from '../lib/ca-publication-model.mjs';

  export let oncommand;

  let state;
  let revocation;
  let publication;
  let downloadError = '';
  const model = createCaOperations({ oncommand: (payload) => oncommand(payload),
    onState: (next) => { state = next; } });
  state = model.snapshot();
  const revocations = createCaRevocation({ oncommand: (payload) => oncommand(payload),
    onState: (next) => { revocation = next; } });
  revocation = revocations.snapshot();
  const publications = createCaPublication({ oncommand: (payload) => oncommand(payload),
    onState: (next) => { publication = next; } });
  publication = publications.snapshot();
  $: busy = state.busy || revocation.busy || publication.busy;
  $: pendingIssue = Boolean(state.operation_id && state.phase !== 'completed');
  $: pendingRevocation = ['submitting', 'uncertain'].includes(revocation.phase);
  $: pendingPublication = ['publishing', 'uncertain'].includes(publication.phase);
  $: pendingOperation = pendingIssue ? state.operation_id : pendingRevocation
    ? revocation.operation_id : pendingPublication ? publication.operation_id : null;
  onDestroy(() => { model.dispose(); revocations.dispose(); publications.dispose(); });

  async function revoke(action) {
    await action();
    if (revocation.phase === 'completed') {
      await model.loadInventory(state.inventory.offset);
      await model.loadHistory(state.history.offset);
    }
  }

  async function publish(action) {
    await action();
    if (publication.phase === 'completed') {
      revocations.cancel();
      await model.loadInventory(state.inventory.offset);
      await model.loadHistory(state.history.offset);
    }
  }

  async function resumeHistory(item) {
    if (pendingOperation && pendingOperation !== item.operation_id) return;
    if (item.kind === 'issue') await model.resume(item.operation_id);
    else if (item.kind === 'revoke') {
      model.setTab('inventory');
      await revoke(() => revocations.resumeOperation(item));
    } else if (item.kind === 'publication') {
      model.setTab('publication');
      await publish(() => publications.resumeOperation(item));
    }
  }

  async function select(tab) {
    downloadError = '';
    model.setTab(tab);
    if (tab === 'inventory' && !state.inventory.loaded) await model.loadInventory();
    if (tab === 'history' && !state.history.loaded) await model.loadHistory();
    if (tab === 'publication' && !publication.status && !publication.operation_id) await publications.refresh();
  }

  async function download(identity) {
    downloadError = '';
    const certificate = await model.download(identity);
    if (!certificate) return;
    let url;
    let link;
    try {
      url = URL.createObjectURL(new Blob([certificate.certificate_pem], { type: 'application/x-pem-file' }));
      link = document.createElement('a');
      link.href = url;
      link.download = `certificado-${certificate.serial}.crt.pem`;
      document.body.append(link);
      link.click();
    } catch {
      downloadError = 'No se pudo iniciar la descarga en este navegador. Puedes volver a intentarlo.';
    } finally {
      link?.remove();
      if (url) window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
  }
</script>

<section class="ca-operations" aria-label="Operaciones de la autoridad certificadora">
  <nav class="ca-tabs" aria-label="Secciones de la autoridad certificadora">
    {#each [['requests', 'Solicitudes', 'file'], ['inventory', 'Certificados', 'shield'], ['history', 'Historial', 'clock'], ['publication', 'Publicaci\u00f3n', 'upload']] as [tab, label, icon]}
      <button type="button" class:active={state.tab === tab} aria-pressed={state.tab === tab}
        disabled={busy || (tab === 'requests' && (pendingRevocation || pendingPublication))} onclick={() => select(tab)}>
        <Icon name={icon} size={18} />{label}
      </button>
    {/each}
  </nav>

  {#if state.error}<div class="notice error" role="alert">{state.error}</div>{/if}
  {#if downloadError}<div class="notice error" role="alert">{downloadError}</div>{/if}

  <CaRevocation state={revocation} busy={state.busy || publication.busy} blocked={pendingIssue || pendingPublication}
    onreason={revocations.setReason} oncancel={revocations.cancel}
    onconfirm={(identity) => revoke(() => revocations.confirm(identity))}
    onresume={() => revoke(revocations.resume)} />

  <div aria-busy={busy}>
    {#if state.tab === 'requests'}
      <CaRequest state={{ ...state, busy: busy || pendingRevocation || pendingPublication }} onedit={model.edit}
        onpreview={model.preview} onconfirm={model.confirm}
        onresume={model.resume} onnew={model.startNew} ondownload={download} />
    {:else if state.tab === 'inventory'}
      <CaCertificates page={state.inventory} {busy} mutationBlocked={Boolean(pendingOperation)}
        onload={model.loadInventory} ondownload={download} onrevoke={revocations.choose} />
    {:else if state.tab === 'history'}
      <CaHistory page={state.history} {busy} blockedOperation={pendingOperation}
        onload={model.loadHistory} onresume={resumeHistory} ondownload={download} />
    {:else}
      <CaPublication state={publication} busy={state.busy || revocation.busy}
        blocked={pendingIssue || pendingRevocation} onrefresh={publications.refresh}
        onconfirm={(value) => publish(() => publications.confirm(value))}
        onresume={() => publish(publications.resume)} onhistory={() => select('history')}
        onreset={() => { publications.reset(); void publications.refresh(); }} />
    {/if}
  </div>
</section>
