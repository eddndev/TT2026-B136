<script>
  import { onDestroy } from 'svelte';
  import { download } from '../lib/documents.mjs';
  import { base64Bytes } from '../lib/typed-participant-preparation.mjs';
  export let api,
    reference,
    ondenied,
    disabled = false,
    busy = false;
  let evidence = null,
    error = '',
    alive = true;
  async function load() {
    if (disabled || busy) return;
    busy = true;
    error = '';
    try {
      const result = await api.credential(reference.participant_id, reference.participant_revision);
      if (!alive) return;
      if (result.statement_digest !== reference.statement_digest)
        throw new Error('La evidencia no corresponde a la declaraci\u00f3n vinculada.');
      evidence = result;
    } catch (failure) {
      if (alive) {
        evidence = null;
        error = failure.message;
        if (
          [403, 404].includes(failure.status) &&
          failure.code !== 'participant_credential_not_found'
        )
          ondenied(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function json() {
    download(
      new Blob([JSON.stringify(evidence, null, 2)], { type: 'application/json' }),
      'evidencia-publica-participante.json',
    );
  }
  function binary(value, name) {
    download(new Blob([base64Bytes(value)], { type: 'application/octet-stream' }), name);
  }
  onDestroy(() => {
    alive = false;
    busy = false;
    evidence = null;
  });
</script>

<div class="participant-history">
  <button class="text-button" disabled={disabled || busy} onclick={load}
    >Consultar evidencia de firma personal</button
  >
  {#if evidence}<section class="participant-comparison" aria-label="Evidencia de firma personal">
      <h3>Firma comprobada con CA interna</h3>
      <p>
        Comprobaci&#243;n capturada: <time
          datetime={new Date(evidence.checked_at_unix * 1000).toISOString()}
          >{new Date(evidence.checked_at_unix * 1000).toLocaleString('es-MX')}</time
        >.
      </p>
      <p>
        Declaraci&#243;n de la revisi&#243;n {reference.participant_revision}. Esta evidencia no
        declara FIREL oficial ni vigencia actual del certificado.
      </p>
      <dl class="participant-values">
        <div>
          <dt>Certificado</dt>
          <dd>{evidence.certificate_summary.subject}</dd>
        </div>
        <div>
          <dt>Emisor</dt>
          <dd>{evidence.certificate_summary.issuer}</dd>
        </div>
        <div>
          <dt>Serie</dt>
          <dd>{evidence.certificate_summary.serial_hex}</dd>
        </div>
        <div>
          <dt>Confianza capturada</dt>
          <dd>Revisi&#243;n {evidence.trust.revision}</dd>
        </div>
      </dl>
      <p class="hint participant-provenance">
        SHA-256 del certificado: {evidence.certificate_fingerprint}<br />SHA-256 de
        declaraci&#243;n: {evidence.statement_digest}
      </p>
      <div class="action-row">
        <button class="secondary" disabled={disabled || busy} onclick={json}
          >Descargar evidencia p&#250;blica</button
        ><button
          class="secondary"
          disabled={disabled || busy}
          onclick={() => binary(evidence.declaration_base64, 'declaracion-participante.bin')}
          >Descargar declaraci&#243;n comprobada</button
        ><button
          class="secondary"
          disabled={disabled || busy}
          onclick={() => binary(evidence.signature_base64, 'declaracion-participante.sig')}
          >Descargar firma comprobada</button
        >
      </div>
    </section>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
</div>
