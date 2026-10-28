<script>
  // Validates a local evidence ZIP in a dedicated Web Worker. The file never
  // leaves the browser; only the trust route is read from the server. A
  // newer run, cancel, a version change or closing the panel discards any
  // late result. Rationale: docs/adr/0085-in-browser-evidence-validation.md.
  import { onDestroy } from 'svelte';
  import { createEvidenceVerifier } from '../lib/evidence-verify-client.mjs';
  import { pinnedRoots } from '../lib/evidence/pinned-roots.mjs';
  import { buildPolicy } from '../lib/evidence/policy.mjs';
  import { MAX_EVIDENCE_FILE_BYTES, STAGE_LABELS } from '../lib/evidence/presentation.mjs';
  import EvidenceValidationResult from './EvidenceValidationResult.svelte';
  export let api;
  export let document;
  const STAGES = ['read', 'trust', 'module', 'validate'];
  const TOO_LARGE =
    'El archivo supera el l\u00edmite de 26 MiB. Elige el ZIP de evidencia que descargaste de Qadra.';
  const verifier = createEvidenceVerifier();
  let alive = true;
  let generation = 0;
  let file = null;
  let stage = '';
  let error = '';
  let message = '';
  let result = null;
  $: context = `${document.id}:${document.version}`;
  $: running = !!stage;

  function clear() {
    error = '';
    message = '';
    result = null;
  }

  function select(event) {
    clear();
    file = event.currentTarget.files?.[0] || null;
    if (file && file.size > MAX_EVIDENCE_FILE_BYTES) {
      error = TOO_LARGE;
      file = null;
    }
  }

  async function sha256(buffer) {
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', buffer));
    return [...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('');
  }

  async function start() {
    if (!file || running) return;
    const current = ++generation;
    const expected = context;
    const chosen = file;
    const live = () => alive && current === generation && expected === context;
    clear();
    stage = 'read';
    try {
      if (chosen.size > MAX_EVIDENCE_FILE_BYTES) throw new Error(TOO_LARGE);
      const zip = await chosen.arrayBuffer();
      const digest = await sha256(zip);
      if (!live()) return;
      stage = 'trust';
      const route = api?.evidenceTrust ? await api.evidenceTrust() : null;
      if (!live()) return;
      const policy = buildPolicy({ pinned: pinnedRoots, trust: route, now: Date.now() });
      const answer = await verifier.run({ context: expected, zip, policy }, (next) => {
        if (live()) stage = next;
      });
      if (!live() || answer.context !== context) return;
      result = {
        report: answer.report,
        policy,
        route,
        document: { id: document.id, version: document.version },
        file: { name: chosen.name, size: chosen.size, sha256: digest },
      };
    } catch (failure) {
      if (!live() || failure?.name === 'AbortError') return;
      error = failure?.message || 'No se pudo validar el archivo.';
    } finally {
      if (alive && current === generation) stage = '';
    }
  }

  function cancel() {
    generation++;
    verifier.cancel();
    stage = '';
    clear();
    message = 'Validaci\u00f3n cancelada. El archivo no se revis\u00f3.';
  }

  onDestroy(() => {
    alive = false;
    generation++;
    verifier.cancel();
  });
</script>

<section class="evidence-validation" aria-labelledby="evidence-validation-title">
  <h4 id="evidence-validation-title">Validar un paquete de evidencia</h4>
  <p class="hint">
    Elige el ZIP de evidencia que descargaste. Se revisa en este navegador y el archivo no se
    env&#237;a a ning&#250;n servidor.
  </p>
  <div class="evidence-validation-form">
    <label for="evidence-validation-file">Paquete de evidencia (ZIP)</label>
    <input
      id="evidence-validation-file"
      type="file"
      accept=".zip,application/zip"
      disabled={running}
      onchange={select}
    />
    <div class="evidence-validation-actions">
      <button class="secondary" disabled={!file || running} onclick={start}>Validar paquete</button>
      {#if running}<button class="secondary" onclick={cancel}>Cancelar</button>{/if}
    </div>
  </div>
  {#if running}<p class="evidence-validation-live" role="status">
      Paso {STAGES.indexOf(stage) + 1} de {STAGES.length}
    </p>
    <ol class="evidence-validation-stages" aria-label="Avance de la validaci&#243;n">
      {#each STAGES as step, index}<li
          class:done={STAGES.indexOf(stage) > index}
          class:current={stage === step}
          aria-current={stage === step ? 'step' : undefined}
        >
          {STAGE_LABELS[step]}
        </li>{/each}
    </ol>{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if message}<p class="notice" role="status">{message}</p>{/if}
  {#if result}<EvidenceValidationResult {...result} pinned={pinnedRoots} />{/if}
</section>
