<script>
  import CalendarFields from './CalendarFields.svelte';
  import CalendarValues from './CalendarValues.svelte';
  export let action, base, draft, prepared, last, candidate, compared, mode, error;
  export let busy, frozen, blocked, prepare, submit, check, compare, accept, close, retry, back;
</script>

<section class="card calendar-editor" aria-label="Formulario de calendario" aria-busy={busy}>
  <h2>
    {action === 'publish'
      ? 'Publicar calendario'
      : action === 'replace'
        ? 'Reemplazar calendario'
        : 'Retirar calendario'}
  </h2>
  <p class="hint">
    Clasificaci&#243;n declarada por &#225;mbito. Esta captura no calcula plazos ni acredita el
    contenido de las fuentes.
  </p>
  {#if blocked}<button class="secondary" disabled={busy} onclick={retry}
      >Volver a consultar el contexto del calendario</button
    >{/if}
  {#if error}<p class="notice error" role="alert">{error}</p>{/if}
  {#if mode === 'uncertain'}<div class="case-comparison">
      <h3>Resultado incierto</h3>
      <p>Consulta el recibo del env&#237;o. La ausencia temporal no confirma que fall&#243;.</p>
      <button class="primary" disabled={busy || blocked} onclick={check}
        >Consultar env&#237;o exacto</button
      >
      <p class="hint">Cerrar descarta el borrador local; no cancela una escritura en curso.</p>
    </div>{/if}
  {#if last}<details>
      <summary>Identidad del &#250;ltimo env&#237;o</summary>
      <p>
        Calendario: <code>{last.command.calendar_id}</code> / Revisi&#243;n objetivo {last.result_revision}
      </p>
      <p>Operaci&#243;n: <code>{last.command.operation_id}</code></p>
      <p>Recibo: <code>{last.submission_digest}</code></p>
    </details>{/if}
  {#if mode === 'conflict'}<div class="case-comparison">
      <h3>Comparar con la base actual</h3>
      <p>
        Tu borrador se conserva. Consulta los valores actuales antes de preparar una nueva
        operaci&#243;n.
      </p>
      <button class="secondary" disabled={busy || blocked} onclick={compare}
        >Consultar base actual del calendario</button
      >
      {#if compared && candidate}<p>
          Revisi&#243;n {candidate.revision} / {candidate.status === 'retired'
            ? 'Retirado'
            : 'Publicado'}
        </p>
        <CalendarValues values={candidate.values} />
        {#if action !== 'publish' && candidate.status === 'published'}<button
            class="primary"
            disabled={busy || blocked}
            onclick={accept}>Usar esta base y conservar borrador</button
          >{:else}<p>
            Esta base no admite reenviar el borrador. Cierra el formulario y consulta su historia.
          </p>{/if}
      {/if}
    </div>{/if}
  {#if prepared}<div class="case-comparison">
      <h3>Revisa el calendario a registrar</h3>
      <p>Revisi&#243;n a registrar: {prepared.result_revision}</p>
      <CalendarValues values={prepared.values} />{#if prepared.command.change.reason}<p
          class="case-multiline"
        >
          Motivo: {prepared.command.change.reason}
        </p>{/if}
      <details>
        <summary>Recibo de la preparaci&#243;n</summary>
        <p>Operaci&#243;n: <code>{prepared.command.operation_id}</code></p>
        <p>Valores: <code>{prepared.values_digest}</code></p>
        <p>Recibo: <code>{prepared.submission_digest}</code></p>
      </details>
      <div class="action-row">
        <button class="secondary" disabled={busy} onclick={back}
          >Volver al borrador del calendario</button
        ><button class="primary" disabled={frozen} onclick={submit}>Confirmar calendario</button>
      </div>
    </div>{:else if mode !== 'confirmed'}
    {#if action === 'retire'}<CalendarValues values={base.values} />
      <p class="notice">
        Retirar conserva todos los valores e historia. Es terminal y no declara derogaci&#243;n.
      </p>
    {:else}<CalendarFields
        bind:draft
        disabled={frozen}
        immutableScope={action !== 'publish'}
      />{/if}
    {#if action !== 'publish'}<label
        >Motivo<textarea rows="3" bind:value={draft.reason} disabled={frozen}></textarea></label
      >{/if}
    <button class="primary" disabled={frozen || mode !== 'draft'} onclick={prepare}
      >Revisar calendario</button
    >
  {/if}
  <button class="text-button" disabled={busy} onclick={close}
    >Cerrar formulario de calendario</button
  >
</section>
