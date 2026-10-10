<script>
  import { onDestroy } from 'svelte';
  import SigningConsentDetails from './SigningConsentDetails.svelte';
  export let api,
    user,
    document,
    disabled = false,
    busy = '';
  let alive = true,
    result = null,
    error = '';
  const currentLabels = {
    valid: 'vigente',
    rejected: 'rechazado',
    trust_unavailable: 'confianza no disponible',
    not_individual: 'no aplica',
  };
  async function verify() {
    if (busy || disabled) return;
    busy = 'personal-verification';
    error = '';
    result = null;
    try {
      const value = await api.individualSigning(user.id, document.digest).verification();
      if (alive) result = value;
    } catch (failure) {
      if (alive) error = failure.message;
    } finally {
      if (alive) busy = '';
    }
  }
  onDestroy(() => (alive = false));
</script>

<button class="secondary" disabled={!!busy || disabled} onclick={verify}
  >Consultar estado de la firma</button
>
{#if error}<p class="notice error" role="alert">{error}</p>{/if}
{#if result}<section class="card stack">
    <h3>{result.kind === 'individual' ? 'Firma individual' : 'Sello del servidor'}</h3>
    <p>
      Validez historica: {result.historical.verdict === 'valid' ? 'valida' : 'requiere revision'}
    </p>
    {#if result.kind === 'individual'}
      <p>Estado actual del certificado: {currentLabels[result.certificate_current.status]}</p>
      {#if result.certificate_current.reason}<p>
          Motivo actual: {result.certificate_current.reason}
        </p>{/if}
      {#if result.binding_withdrawn_at}<p>Vinculo retirado posteriormente</p>
        <p>{result.binding_withdrawn_at}</p>{/if}
      <p>El estado actual y el retiro no cambian la evidencia historica capturada.</p>
      <SigningConsentDetails value={result.signing} />
    {:else}<p>Esta evidencia corresponde al sellado del servidor.</p>{/if}
    <p>
      El sello de tiempo acredita el resumen del contenido; no demuestra por si solo cuando existio
      la firma personal.
    </p>
  </section>{/if}
