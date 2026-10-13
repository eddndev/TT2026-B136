<script>
  import { getContext, onMount, onDestroy } from 'svelte';
  import CalendarEditorBody from './CalendarEditorBody.svelte';
  import {
    createCalendarDraft,
    freshCalendarOwner,
    denyCalendarDrafts,
  } from '../lib/calendar-draft.mjs';
  import { captureCalendarDraft } from '../lib/calendar-draft-values.mjs';
  import { calendarDraft, calendarCommand } from '../lib/judicial-calendar-values.mjs';
  import {
    calendarDenied,
    calendarUncertain,
    calendarFailure,
  } from '../lib/judicial-calendar-labels.mjs';
  import { readCalendarSubmission } from '../lib/judicial-calendar-submission.mjs';
  export let api,
    user,
    action,
    record = null,
    onconfirmed,
    oncancel,
    ondenied,
    busy = false,
    savedDraft = null;
  const scoped = api.judicialCalendars(),
    session = getContext('session-drafts');
  let calendarId = record?.id || crypto.randomUUID();
  let base = record,
    draft = calendarDraft(record),
    prepared = null,
    last = null,
    candidate = null,
    compared = false,
    mode = 'draft',
    error = '',
    alive = true;
  let blocked = true,
    loaded = false,
    finished = false;
  const recovery = session
    ? createCalendarDraft({
        session,
        principalId: user.id,
        capture: () => captureCalendarDraft({ id: calendarId, action, base, draft, mode, last }),
      })
    : null;
  function admitted() {
    return alive && !finished && (!recovery || recovery.admitted());
  }
  function fresh(resourceId = savedDraft?.resourceId ?? base?.id ?? null) {
    return freshCalendarOwner({ api, scoped, principalId: user.id, resourceId, admitted });
  }
  async function initialize() {
    if (busy || !admitted()) return;
    blocked = busy = true;
    error = '';
    try {
      let context;
      if (savedDraft && !loaded && recovery) {
        const outcome = await recovery.restore(savedDraft, fresh, (value, current) => {
          ({ id: calendarId, action, base, draft, mode, last } = value);
          loaded = true;
          context = current;
        });
        if (!admitted()) return;
        if (outcome.status !== 'restored')
          throw new Error('No se pudo recuperar el borrador del calendario.');
      } else {
        context = await fresh();
        if (!context || !admitted()) return;
        if (!loaded) {
          recovery?.register(action, base?.id ?? null, base?.revision ?? 0);
          loaded = true;
        }
      }
      if (context.current && context.current.revision < base.revision)
        throw new Error('La cabeza actual no confirma la revision original del calendario.');
      if (
        mode !== 'uncertain' &&
        ((context.current?.revision ?? 0) !== (base?.revision ?? 0) ||
          context.current?.status === 'retired')
      )
        mode = 'conflict';
      blocked = false;
    } catch (failure) {
      if (admitted()) {
        error = calendarFailure(failure);
        if (calendarDenied(failure) || failure.status === 404) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function deny(failure) {
    if (calendarDenied(failure)) denyCalendarDrafts(session);
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    recovery?.close();
    blocked = true;
    finished = true;
    ondenied(failure);
  }
  function close() {
    if (busy) return;
    if (savedDraft) session?.registry.closeEditor(savedDraft.key);
    recovery?.close();
    finished = true;
    oncancel();
  }
  $: frozen =
    busy || blocked || !admitted() || ['uncertain', 'exhausted', 'confirmed'].includes(mode);
  function fail(failure, writing = false) {
    if (calendarDenied(failure)) {
      deny(failure);
      return;
    }
    error = calendarFailure(failure);
    prepared = null;
    if (
      writing &&
      (calendarUncertain(failure) || failure.code === 'judicial_calendar_operation_conflict')
    ) {
      mode = 'uncertain';
      error =
        'No se pudo confirmar el resultado. Conservamos el env\u00edo para consultar su revisi\u00f3n exacta.';
    } else if (failure.code === 'judicial_calendar_revision_exhausted') mode = 'exhausted';
    else if (
      [
        'judicial_calendar_revision_conflict',
        'judicial_calendar_retired',
        'judicial_calendar_not_found',
      ].includes(failure.code)
    ) {
      mode = 'conflict';
      compared = false;
    } else mode = 'draft';
  }
  async function prepare() {
    if (!admitted() || frozen || mode !== 'draft') return;
    busy = true;
    error = '';
    try {
      const command = calendarCommand(draft, {
        action,
        base,
        calendarId,
        operationId: crypto.randomUUID(),
      });
      const row = await scoped.prepare(command, user.id);
      if (admitted()) {
        prepared = structuredClone(row);
        mode = 'review';
      }
    } catch (failure) {
      if (admitted()) fail(failure);
    } finally {
      if (alive) busy = false;
    }
  }
  async function finish(row, exact = false) {
    recovery?.close();
    finished = true;
    mode = 'confirmed';
    prepared = null;
    try {
      await onconfirmed(row, exact);
    } catch {
      if (alive)
        error =
          'El calendario se guard\u00f3. Vuelve a consultar la lista sin reenviar la escritura.';
    }
  }
  async function submit() {
    if (!admitted() || frozen || mode !== 'review' || !prepared) return;
    busy = true;
    error = '';
    last = structuredClone(prepared);
    mode = 'uncertain';
    try {
      const row = await scoped.submit(last);
      if (admitted()) await finish(row);
    } catch (failure) {
      if (admitted()) fail(failure, true);
    } finally {
      if (alive) busy = false;
    }
  }
  async function check() {
    if (busy || blocked || !admitted() || !last) return;
    busy = true;
    error = '';
    try {
      const result = await readCalendarSubmission(scoped, last);
      if (!admitted()) return;
      if (result.state === 'matched') await finish(result.record, true);
      else if (result.state === 'absent')
        error =
          'La revisi\u00f3n a\u00fan no est\u00e1 disponible. El resultado sigue incierto; puedes consultar de nuevo.';
      else {
        mode = 'conflict';
        candidate = result.record;
        compared = false;
        error = 'La revisi\u00f3n corresponde a otro env\u00edo. Tu borrador se conserva.';
      }
    } catch (failure) {
      if (admitted()) {
        error = calendarFailure(failure);
        if (calendarDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  async function compare() {
    if (busy || blocked || !admitted()) return;
    busy = true;
    error = '';
    compared = false;
    try {
      const context = await fresh(calendarId);
      if (context && admitted()) {
        const row = context.current;
        candidate = row;
        compared = true;
      }
    } catch (failure) {
      if (admitted()) {
        error = calendarFailure(failure);
        if (calendarDenied(failure)) deny(failure);
      }
    } finally {
      if (alive) busy = false;
    }
  }
  function accept() {
    if (
      !admitted() ||
      blocked ||
      !compared ||
      candidate?.status !== 'published' ||
      action === 'publish' ||
      busy
    )
      return;
    base = candidate;
    recovery?.register(action, base.id, base.revision);
    mode = 'draft';
    compared = false;
    error = '';
  }
  onMount(initialize);
  onDestroy(() => {
    recovery?.dispose();
    alive = false;
    busy = false;
    scoped.dispose();
  });
</script>

<CalendarEditorBody
  {action}
  {base}
  bind:draft
  {prepared}
  {last}
  {candidate}
  {compared}
  {mode}
  {error}
  {busy}
  {frozen}
  {blocked}
  {prepare}
  {submit}
  {check}
  {compare}
  {accept}
  {close}
  retry={initialize}
  back={() => {
    if (admitted() && !busy) {
      prepared = null;
      mode = 'draft';
    }
  }}
/>
