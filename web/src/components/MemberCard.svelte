<script>
  import { roles } from '../lib/documents.mjs';
  export let record,
    onopen,
    disabled = false,
    assignment = null;
  $: action =
    assignment === 'assigned'
      ? 'Retirar'
      : assignment === 'available'
        ? 'Asignar'
        : 'Administrar acceso de';
</script>

<article class="member-card" data-user-id={record.id}>
  <div class="member-identification">
    <h3>{record.email}</h3>
    <div class="member-badges">
      <span class="badge info">{roles[record.role]}</span>
      <span class="badge" class:warning={!record.active}
        >{record.active ? 'Activa' : 'Inactiva'}</span
      >
    </div>
    <p class="hint">Revisi&#243;n {record.revision}</p>
    {#if record.assigned_at}<p class="hint">
        Asignada: <time datetime={record.assigned_at}
          >{record.assigned_at.replace('T', ' ').replace(/(?:Z|\+00:00)$/, ' UTC')}</time
        >
      </p>{/if}
  </div>
  <button class="secondary" {disabled} onclick={() => onopen(record)}
    >{action} {record.email}</button
  >
</article>

<style>
  .member-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    padding-block: 18px;
    border-bottom: 1px solid var(--line);
    min-width: 0;
  }
  .member-identification {
    min-width: 0;
  }
  h3 {
    font-size: 15px;
    margin: 0 0 10px;
    overflow-wrap: anywhere;
  }
  .member-badges {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  p {
    margin: 8px 0 0;
    overflow-wrap: anywhere;
  }
  button {
    white-space: normal;
    overflow-wrap: anywhere;
    max-width: 100%;
  }
  @media (max-width: 700px) {
    .member-card {
      align-items: stretch;
      flex-direction: column;
    }
  }
</style>
