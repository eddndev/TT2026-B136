<script>
  import DeadlineFields from './DeadlineFields.svelte';
  import DeadlineFieldsAttention from './DeadlineFieldsAttention.svelte';
  import DeadlineFieldsReview from './DeadlineFieldsReview.svelte';
  import CaseClosedNotice from './CaseClosedNotice.svelte';
  export let api,
    caseId,
    mode,
    pending,
    frozen,
    step,
    error,
    blocked,
    closed,
    fieldsVersion,
    prepared,
    last,
    candidate,
    compared,
    inputs,
    recoverable,
    definition,
    policies,
    attention,
    reason,
    profile,
    responsible,
    acknowledge,
    fieldsBusy,
    fieldsDisabled,
    ondenied,
    actions,
    oncontext,
    onclose,
    onback;
  let fields, attentionFields;
  export function captureInputs() {
    return {
      definition: fields?.captureInputs() ?? inputs?.definition ?? null,
      attention: attentionFields?.captureInputs() ?? inputs?.attention ?? null,
    };
  }
</script>

<section class="card case-editor fact-editor" aria-label="Formulario de plazo" aria-busy={pending}>
  <h2>
    {mode === 'register'
      ? 'Registrar plazo'
      : mode === 'correct'
        ? 'Corregir plazo'
        : mode === 'set_attention'
          ? 'Declarar atencion'
          : 'Retirar plazo'}
  </h2>
  <p class="hint">
    La captura conserva una evaluaci&#243;n de datos declarados. La revisi&#243;n de aplicabilidad
    corresponde a la persona operadora.
  </p>
  <CaseClosedNotice />
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if blocked || closed}
    <button class="secondary" disabled={pending} onclick={oncontext}
      >Volver a consultar el contexto</button
    >
  {/if}
  {#if step === 'uncertain'}
    <section class="case-comparison" aria-label="Resultado incierto del plazo">
      <h3>Resultado incierto</h3>
      <p>Una ausencia temporal no confirma que la escritura haya fallado.</p>
      <button class="primary" disabled={pending || blocked || !last} onclick={actions.check}
        >Consultar envio exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </section>
  {/if}
  {#if step === 'conflict'}
    <section class="case-comparison" aria-label="Comparar base del plazo">
      <h3>Comparar con el plazo actual</h3>
      <button class="secondary" disabled={pending || blocked} onclick={actions.compare}
        >Consultar base actual</button
      >
      {#if compared && candidate}
        <p>
          Base consultada: revisi&#243;n {candidate.revision} / {candidate.status === 'retired'
            ? 'Retirado'
            : 'Activo'}
        </p>
        <DeadlineFieldsReview value={candidate} />
        {#if candidate.status === 'active' && mode !== 'register'}
          <button class="primary" disabled={frozen} onclick={actions.accept}
            >Usar esta base y conservar borrador</button
          >
        {:else}
          <p>
            Esta captura no permite reenviar sobre la base consultada. Cierra el formulario para
            revisar su historia.
          </p>
        {/if}
      {/if}
    </section>
  {/if}
  {#if prepared}
    <section class="case-comparison" aria-label="Revision del plazo a confirmar">
      <h3>Revisa el plazo a confirmar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <DeadlineFieldsReview value={prepared} {profile} />
      {#if prepared.command.change.reason}<p class="case-multiline">
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <label class="checkbox"
        ><input type="checkbox" bind:checked={acknowledge} disabled={frozen} />
        {prepared.calculation.result.blocks.length
          ? 'Revise las declaraciones, fuentes y politicas; confirmo guardar el plazo con estos bloqueos'
          : 'Revise las declaraciones, fuentes, politicas y el resultado a guardar'}</label
      >
      <div class="action-row">
        <button class="secondary" disabled={pending} onclick={onback}>Volver al borrador</button>
        <button class="primary" disabled={frozen || !acknowledge} onclick={actions.submit}
          >Confirmar plazo</button
        >
      </div>
    </section>
  {:else if step !== 'confirmed'}
    {#key fieldsVersion}
      {#if ['register', 'correct'].includes(mode)}
        <DeadlineFields
          bind:this={fields}
          {api}
          {caseId}
          {ondenied}
          {recoverable}
          savedInputs={inputs?.definition}
          bind:value={definition}
          bind:profile
          bind:responsible
          bind:policies
          bind:pending={fieldsBusy}
          disabled={fieldsDisabled}
        />
      {:else if mode === 'set_attention'}
        <DeadlineFieldsAttention
          bind:this={attentionFields}
          bind:value={attention}
          {recoverable}
          savedInputs={inputs?.attention}
          disabled={frozen}
        />
      {:else}
        <p class="notice">
          El retiro es terminal. Conserva el c&#225;lculo y la historia; no anula el acto ni declara
          extinguido el plazo.
        </p>
      {/if}
    {/key}
    {#if mode !== 'register'}
      <label
        >Motivo<textarea rows="3" maxlength="1000" bind:value={reason} disabled={frozen}
        ></textarea></label
      >
    {/if}
    <button class="primary" disabled={frozen || step !== 'draft'} onclick={actions.prepare}
      >Preparar plazo</button
    >
  {/if}
  <button class="text-button" disabled={pending} onclick={onclose}>Cerrar formulario</button>
</section>
