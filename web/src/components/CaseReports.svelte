<script>
  import { onMount, onDestroy } from 'svelte';
  import CaseReportRequest from './CaseReportRequest.svelte';
  import CaseReportList from './CaseReportList.svelte';
  import CaseReportDetail from './CaseReportDetail.svelte';
  import {
    canReports,
    reportScope,
    scopeLabel,
    initialReportFilters,
    reportFilters,
    reportFailure,
  } from '../lib/case-reports-presentation.mjs';
  import '../styles/case-reports.css';
  export let api, user;
  let mounted = false,
    alive = true,
    seen = null,
    epoch = 0;
  let scoped = null,
    listRevision = 0,
    detailRevision = 0,
    pickerRevision = 0;
  let rows = [],
    more = false,
    next = null,
    loaded = false,
    listBusy = false;
  let lawyers = [],
    pickerBusy = false,
    pickerError = '',
    pickerMore = false,
    pickerNext = null,
    error = '';
  let draft = initialReportFilters(),
    pending = null,
    requestBusy = false,
    requestError = '';
  let selected = null,
    selectedId = null,
    detailBusy = false,
    downloading = '',
    reading = false;
  const urls = new Set();
  $: allowed = canReports(user?.role);
  $: identity = `${user?.id}:${user?.role}`;
  $: if (mounted && identity !== seen) initialize();
  const current = (context) => alive && context === epoch && identity === seen;
  function releaseUrls() {
    for (const url of urls) URL.revokeObjectURL(url);
    urls.clear();
  }
  function clear() {
    epoch++;
    listRevision++;
    detailRevision++;
    pickerRevision++;
    rows = [];
    more = false;
    next = null;
    loaded = false;
    lawyers = [];
    pickerMore = false;
    pickerNext = null;
    selected = null;
    selectedId = null;
    pending = null;
    listBusy = false;
    detailBusy = false;
    requestBusy = false;
    pickerBusy = false;
    reading = false;
    downloading = '';
    releaseUrls();
  }
  function initialize() {
    scoped?.dispose();
    clear();
    seen = identity;
    draft = initialReportFilters();
    error = '';
    requestError = '';
    pickerError = '';
    scoped = allowed ? api.reports() : null;
    if (scoped) {
      load();
      loadPicker();
    }
  }
  function scopeCheck(value) {
    if (value.scope !== reportScope(user.role))
      throw new Error('El alcance del informe no corresponde a tu acceso actual.');
    return value;
  }
  function receive(value) {
    selected = scopeCheck(value);
    selectedId = value.id;
    rows = rows.some((row) => row.id === value.id)
      ? rows.map((row) => (row.id === value.id ? value : row))
      : rows;
  }
  function fail(failure) {
    if ([403, 404].includes(failure.status)) {
      clear();
      error = reportFailure(failure);
    } else error = reportFailure(failure);
  }
  async function load(append = false) {
    if (!scoped || listBusy || (append && !more)) return;
    const context = epoch,
      revision = ++listRevision;
    listBusy = true;
    error = '';
    if (!append) {
      rows = [];
      more = false;
      next = null;
      loaded = false;
      selected = null;
      selectedId = null;
      detailRevision++;
      detailBusy = false;
      reading = false;
      downloading = '';
      releaseUrls();
    }
    try {
      const value = await scoped.list({ limit: 20, ...(append ? { after_id: next } : {}) });
      if (!current(context) || revision !== listRevision) return;
      value.reports.forEach(scopeCheck);
      rows = append
        ? [...new Map([...rows, ...value.reports].map((row) => [row.id, row])).values()]
        : value.reports;
      more = value.has_more;
      next = value.next_after_id;
      loaded = true;
    } catch (failure) {
      if (current(context) && revision === listRevision) fail(failure);
    } finally {
      if (current(context) && revision === listRevision) listBusy = false;
    }
  }
  async function loadPicker(append = false) {
    if (!scoped || pickerBusy || requestBusy || (append && !pickerMore)) return;
    const context = epoch,
      revision = ++pickerRevision;
    pickerBusy = true;
    pickerError = '';
    if (!append) {
      lawyers = [];
      pickerMore = false;
      pickerNext = null;
    }
    try {
      const value = await scoped.litigators({
        limit: 20,
        ...(append ? { after_id: pickerNext } : {}),
      });
      if (!current(context) || revision !== pickerRevision) return;
      scopeCheck(value);
      lawyers = append ? [...lawyers, ...value.litigators] : value.litigators;
      pickerMore = value.has_more;
      pickerNext = value.next_after_id;
      if (draft.assigned && !lawyers.some((row) => row.user_id === draft.assigned))
        draft = { ...draft, assigned: '' };
    } catch (failure) {
      if (current(context) && revision === pickerRevision) {
        pickerError = reportFailure(failure);
        if (failure.status === 403) fail(failure);
      }
    } finally {
      if (current(context) && revision === pickerRevision) pickerBusy = false;
    }
  }
  async function open(id) {
    if (!scoped || detailBusy || downloading || reading) return;
    const context = epoch,
      revision = ++detailRevision;
    selectedId = id;
    selected = null;
    detailBusy = true;
    error = '';
    releaseUrls();
    try {
      const value = await scoped.get(id);
      if (!current(context) || revision !== detailRevision) return;
      receive(value);
    } catch (failure) {
      if (current(context) && revision === detailRevision) fail(failure);
    } finally {
      if (current(context) && revision === detailRevision) detailBusy = false;
    }
  }
  async function request(retry = false) {
    if (!scoped || requestBusy || pickerBusy || pickerError) return;
    let command;
    try {
      if (retry) command = pending;
      else {
        const filters = reportFilters(draft, lawyers);
        command =
          pending && JSON.stringify(pending.filters) === JSON.stringify(filters)
            ? pending
            : { operation_id: crypto.randomUUID(), filters };
      }
      if (!command) return;
    } catch (failure) {
      requestError = reportFailure(failure);
      return;
    }
    const context = epoch;
    requestBusy = true;
    requestError = '';
    error = '';
    pending = command;
    detailRevision++;
    selected = null;
    selectedId = null;
    detailBusy = false;
    reading = false;
    downloading = '';
    releaseUrls();
    try {
      const value = await scoped.request(command);
      if (!current(context)) return;
      scopeCheck(value);
      receive(value);
      pending = null;
      if (!rows.some((row) => row.id === value.id)) rows = [value, ...rows];
    } catch (failure) {
      if (current(context)) {
        requestError = reportFailure(failure);
        if ([403, 404].includes(failure.status)) fail(failure);
        else if (failure.status && failure.status < 500) pending = null;
      }
    } finally {
      if (current(context)) requestBusy = false;
    }
  }
  async function acknowledge() {
    if (!selected?.notice || selected.notice.read_at || reading || downloading || detailBusy)
      return;
    const context = epoch,
      revision = detailRevision,
      id = selected.id;
    reading = true;
    error = '';
    try {
      const value = await scoped.acknowledge(id);
      if (current(context) && revision === detailRevision && selectedId === id) receive(value);
    } catch (failure) {
      if (current(context) && revision === detailRevision) fail(failure);
    } finally {
      if (current(context) && revision === detailRevision) reading = false;
    }
  }
  async function download(format) {
    if (!selected?.ready || downloading || reading || detailBusy) return;
    const context = epoch,
      revision = detailRevision,
      expected = selected;
    downloading = format;
    error = '';
    let url, anchor;
    try {
      const value = await scoped.download(expected.id, format, expected);
      if (!current(context) || revision !== detailRevision || selected !== expected) return;
      if (!(value.blob instanceof Blob) || value.filename !== `report-${expected.id}.${format}`)
        throw new Error('La descarga no corresponde al informe seleccionado.');
      url = URL.createObjectURL(value.blob);
      urls.add(url);
      anchor = document.createElement('a');
      anchor.href = url;
      anchor.download = value.filename;
      document.body.append(anchor);
      anchor.click();
    } catch (failure) {
      if (current(context) && revision === detailRevision) fail(failure);
    } finally {
      anchor?.remove();
      if (url) {
        URL.revokeObjectURL(url);
        urls.delete(url);
      }
      if (current(context) && revision === detailRevision) downloading = '';
    }
  }
  onMount(() => {
    mounted = true;
  });
  onDestroy(() => {
    alive = false;
    clear();
    scoped?.dispose();
  });
</script>

{#if allowed}
  <section
    class="reports-page"
    aria-label="Informes de expedientes"
    aria-busy={listBusy || pickerBusy}
  >
    <div class="page-heading">
      <div>
        <span class="eyebrow">SEGUIMIENTO DEL DESPACHO</span>
        <h1>Informes de expedientes</h1>
        <p>Estado actual de los expedientes creados durante el periodo seleccionado.</p>
      </div>
      <button class="secondary" disabled={listBusy || requestBusy} onclick={() => load()}
        >Actualizar informes</button
      >
    </div>
    <p class="report-scope">{scopeLabel(reportScope(user.role))}</p>
    <p class="notice">
      Los informes conservan una captura administrativa. El periodo filtra la creaci&#243;n del
      expediente; no mide la actividad del periodo ni reconstruye su estado pasado.
    </p>
    {#if error}<p class="notice error" role="alert">{error}</p>{/if}
    <div class="report-layout">
      <CaseReportRequest
        bind:draft
        {lawyers}
        {pickerBusy}
        {pickerError}
        {pickerMore}
        busy={requestBusy}
        error={requestError}
        {pending}
        onrequest={() => request()}
        onretry={() => request(true)}
        onrefresh={() => loadPicker()}
        onmore={() => loadPicker(true)}
      />
      <CaseReportList
        {rows}
        busy={listBusy}
        {loaded}
        {more}
        onopen={open}
        onmore={() => load(true)}
      />
    </div>
    {#if selectedId}<CaseReportDetail
        value={selected}
        busy={detailBusy}
        {downloading}
        {reading}
        onrefresh={() => open(selectedId)}
        ondownload={download}
        onread={acknowledge}
      />{/if}
  </section>
{/if}
