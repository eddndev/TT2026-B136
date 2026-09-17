<script>
  import HearingList from './HearingList.svelte';
  export let rows = [],
    offset = '+00:00',
    onselect,
    disabled = false;
  $: minutes =
    (Number(offset.slice(1, 3)) * 60 + Number(offset.slice(4))) * (offset[0] === '-' ? -1 : 1);
  $: groups = rows.reduce((result, row) => {
    const day = new Date(Date.parse(row.scheduled_at) + minutes * 60000).toISOString().slice(0, 10);
    const group = result.at(-1);
    if (group?.day === day) group.rows.push(row);
    else result.push({ day, rows: [row] });
    return result;
  }, []);
</script>

<div class="hearing-agenda-days">
  {#each groups as group (group.day)}<section aria-label={`Audiencias del ${group.day}`}>
      <h3>{group.day} / UTC{offset}</h3>
      <HearingList rows={group.rows} showCase {onselect} {disabled} />
    </section>{/each}
</div>
