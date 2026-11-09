<script>
  import Button from './Button.svelte';
  // The printable fingerprint sheet of a locally generated key: the SHA-256
  // of its public key (SPKI), the account e-mail and the pinned root, to be
  // signed by hand in two copies. Printing uses the browser only; nothing is
  // sent anywhere. Rationale: docs/adr/0086-bilateral-contract-signing.md.
  import { fingerprintGroups } from '../lib/key-request-values.mjs';
  import { pinnedRoots } from '../lib/evidence/pinned-roots.mjs';
  export let spkiSha256, commonName, organization, country, email;
  $: root =
    pinnedRoots.find((value) => value.purposes.includes('individual-signing')) ?? pinnedRoots[0];
  const copies = ['Ejemplar del Cliente', 'Ejemplar del despacho'];
  const today = new Date().toLocaleDateString('es-MX', { dateStyle: 'long' });
</script>

<section class="fingerprint-panel stack" aria-label="Huella de la clave">
  <h3>Huella de tu clave p&#250;blica</h3>
  <p>
    Lee esta huella en persona o por tel&#233;fono al responsable de la CA. Debe coincidir con la
    que ve al revisar tu solicitud antes de emitir el certificado.
  </p>
  <p class="fingerprint-value" data-testid="spki-sha256">
    <code>{fingerprintGroups(spkiSha256)}</code>
  </p>
  <Button variant="secondary" type="button" onclick={() => window.print()}
    >Imprimir hoja de huella</Button
  >
  <p class="hint">
    Imprime y firma a mano dos ejemplares. La hoja ata a la persona con su clave fuera del sistema;
    no acredita identidad civil.
  </p>
  <div class="fingerprint-sheets">
    {#each copies as copy, index}<article class="fingerprint-sheet" class:print-only={index > 0}>
        <h2>Hoja de huella de firma documental</h2>
        <p class="copy">{copy}</p>
        <dl>
          <dt>Folio</dt>
          <dd class="blank">&nbsp;</dd>
          <dt>Correo de la cuenta</dt>
          <dd>{email || 'Sin correo'}</dd>
          <dt>Titular declarado</dt>
          <dd>{commonName} / {organization} / {country}</dd>
          <dt>SHA-256 de la clave p&#250;blica (SPKI), le&#237;do de este equipo</dt>
          <dd><code>{fingerprintGroups(spkiSha256)}</code></dd>
          <dt>Huella de la ra&#237;z ({root?.label ?? 'sin ra\u00edz fijada'})</dt>
          <dd><code>{root ? fingerprintGroups(root.fingerprint) : ''}</code></dd>
          <dt>Fecha</dt>
          <dd>{today}</dd>
        </dl>
        <div class="signatures">
          <p>Firma del Cliente</p>
          <p>Firma por el despacho</p>
        </div>
      </article>{/each}
  </div>
</section>

<style>
  .fingerprint-panel {
    border: 1px solid var(--color-line);
    border-radius: 12px;
    padding: 16px;
  }
  .fingerprint-value code,
  dd code {
    overflow-wrap: anywhere;
    font-size: 1.05em;
  }
  .fingerprint-sheet {
    border-top: 1px dashed var(--color-line);
    padding-top: 12px;
  }
  .print-only {
    display: none;
  }
  dl {
    display: grid;
    gap: 6px;
  }
  dt {
    font-weight: 500;
  }
  dd {
    margin: 0;
  }
  .blank {
    border-bottom: 1px solid currentColor;
    min-height: 1.5em;
  }
  .signatures {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
    margin-top: 48px;
  }
  .signatures p {
    border-top: 1px solid currentColor;
    padding-top: 6px;
  }
  @media print {
    :global(body *) {
      visibility: hidden;
    }
    .fingerprint-sheets,
    .fingerprint-sheets :global(*) {
      visibility: visible;
    }
    .fingerprint-sheets {
      position: absolute;
      inset: 0 auto auto 0;
      width: 100%;
    }
    .print-only {
      display: block;
    }
    .fingerprint-sheet {
      break-after: page;
      border: 0;
    }
  }
</style>
