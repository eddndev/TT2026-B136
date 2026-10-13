<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import OwnerCertificateForm from './OwnerCertificateForm.svelte';
  import { download, safeFilename } from '../lib/documents.mjs';
  import {
    readOwnerPublicCertificate,
    ownerStatementDownload,
  } from '../lib/owner-certificate-files.mjs';
  import { readDetachedSignature } from '../lib/participant-credential-files.mjs';
  import {
    registrationIntent,
    reconcileOwnerIntent,
    uncertainOwnerWrite,
  } from '../lib/owner-certificate-intent.mjs';
  import {
    createOwnerBindingDraft,
    freshOwnerBinding,
    denyOwnerDrafts,
    ownerFailure,
    ownerBlobBase64,
  } from '../lib/owner-binding-draft.mjs';
  export let api,
    scoped,
    principalId,
    bindingId,
    saved = null,
    onconfirmed,
    oncancel,
    ondenied;
  const session = getContext('session-drafts');
  let prepared = null,
    signatureBase64 = '',
    signatureName = '',
    command = null;
  let selected = null,
    selectedName = '',
    mode = 'editing',
    latest = null;
  let busy = true,
    ready = false,
    alive = true,
    completed = false,
    generation = 0,
    error = '';
  const recovery = createOwnerBindingDraft({
    session,
    principalId,
    bindingId,
    action: 'register',
    capture,
  });
  const admitted = () => alive && !completed && recovery.admitted();
  const active = (request) => admitted() && request === generation;
  $: blocked = latest !== null && latest.binding_id !== bindingId;
  function capture() {
    return {
      prepared,
      signatureBase64,
      signatureName,
      command,
      mode: mode === 'absent' ? 'uncertain' : mode,
      original: null,
    };
  }
  function denied(failure) {
    if (failure.status !== 403) return;
    denyOwnerDrafts(session);
    completed = true;
    ondenied(failure);
  }
  function fresh(request) {
    return freshOwnerBinding(api, scoped, principalId, () => active(request));
  }
  async function load() {
    const request = ++generation;
    busy = true;
    error = '';
    try {
      if (saved) {
        const outcome = await recovery.restore(
          saved,
          () => fresh(request),
          (value, context) => {
            prepared = value.prepared;
            signatureBase64 = value.signatureBase64;
            signatureName = value.signatureName;
            command = value.command;
            mode = value.mode;
            latest = context.current;
          },
        );
        if (!active(request)) return;
        if (outcome.status !== 'restored') throw new Error('No se pudo recuperar el borrador.');
      } else {
        const context = await fresh(request);
        if (!context || !active(request)) return;
        latest = context.current;
        recovery.register();
      }
      ready = true;
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function certificate(event) {
    const file = event.currentTarget.files[0];
    event.currentTarget.value = '';
    if (!ready || busy || !admitted() || prepared || mode !== 'editing') return;
    selected = null;
    selectedName = '';
    error = '';
    if (!file) return;
    const request = ++generation;
    busy = true;
    try {
      const blob = await readOwnerPublicCertificate(file);
      if (active(request)) {
        selected = blob;
        selectedName = safeFilename(file.name);
      }
    } catch {
      if (active(request))
        error =
          'Selecciona un solo certificado publico PEM, hasta 16 KiB, sin claves privadas ni otros bloques.';
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function prepare() {
    if (!ready || busy || !admitted() || !selected || prepared || blocked) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      const encoded = await ownerBlobBase64(selected);
      if (!active(request)) return;
      const value = await scoped.prepare(bindingId, encoded);
      if (!active(request)) return;
      prepared = value;
      selected = null;
      selectedName = '';
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function signature(event) {
    const file = event.currentTarget.files[0];
    event.currentTarget.value = '';
    if (!ready || busy || !admitted() || !prepared || mode !== 'editing') return;
    signatureBase64 = '';
    signatureName = '';
    command = null;
    error = '';
    if (!file) return;
    const request = ++generation;
    busy = true;
    try {
      const blob = await readDetachedSignature(file);
      if (!active(request)) return;
      const encoded = await ownerBlobBase64(blob);
      if (!active(request)) return;
      registrationIntent(prepared, encoded);
      signatureBase64 = encoded;
      signatureName = safeFilename(file.name);
    } catch {
      if (active(request))
        error = 'Selecciona una firma binaria separada de exactamente 384 bytes.';
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function confirmed(receipt) {
    if (reconcileOwnerIntent(command, receipt) !== 'matched')
      throw new Error('El recibo no corresponde al intento original.');
    recovery.close();
    completed = true;
    generation++;
    onconfirmed(receipt);
  }
  async function submit(retry = false) {
    if (
      !ready ||
      busy ||
      !admitted() ||
      blocked ||
      !prepared ||
      !signatureBase64 ||
      (retry ? mode !== 'absent' || !command : mode !== 'editing')
    )
      return;
    if (!retry) command = registrationIntent(prepared, signatureBase64);
    const request = ++generation;
    busy = true;
    mode = 'uncertain';
    error = '';
    try {
      const value = await scoped.register(bindingId, command.data);
      if (active(request)) confirmed(value);
    } catch (failure) {
      if (active(request)) {
        mode = uncertainOwnerWrite(failure) ? 'uncertain' : 'rejected';
        error = ownerFailure(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  async function check() {
    if (!ready || busy || !admitted() || !command) return;
    const request = ++generation;
    busy = true;
    error = '';
    try {
      let value = null;
      try {
        value = await scoped.get(bindingId);
      } catch (failure) {
        if (failure.status !== 404) throw failure;
      }
      if (!active(request)) return;
      const result = reconcileOwnerIntent(command, value);
      if (result === 'matched') {
        confirmed(value);
        return;
      }
      mode = result === 'absent' ? 'absent' : 'conflict';
      error =
        result === 'absent'
          ? 'La consulta no encontro este registro. Puedes reenviar explicitamente el mismo intento.'
          : 'El recibo no coincide con la evidencia original. El envio sigue bloqueado.';
    } catch (failure) {
      if (active(request)) {
        error = ownerFailure(failure);
        denied(failure);
      }
    } finally {
      if (alive && request === generation) busy = false;
    }
  }
  function close() {
    if (busy || !admitted()) return;
    recovery.close();
    completed = true;
    generation++;
    oncancel();
  }
  function statement() {
    if (admitted() && ready && prepared)
      download(ownerStatementDownload(prepared.statement_base64), `vinculo-${bindingId}.bin`);
  }
  onMount(load);
  onDestroy(() => {
    alive = false;
    generation++;
    selected = null;
    recovery.dispose();
  });
</script>

<OwnerCertificateForm
  {prepared}
  {signatureBase64}
  {signatureName}
  {selectedName}
  selected={!!selected}
  {command}
  {latest}
  {bindingId}
  {ready}
  {busy}
  {mode}
  {error}
  onload={load}
  onclose={close}
  oncertificate={certificate}
  onprepare={prepare}
  onsignature={signature}
  onstatement={statement}
  oncheck={check}
  onregister={() => submit()}
  onretry={() => submit(true)}
/>
