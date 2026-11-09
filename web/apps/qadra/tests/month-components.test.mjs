import { test } from 'node:test';
import assert from 'node:assert/strict';
import { componentMarkup as rendered } from './deadline-editor-render.mjs';
import { civilLabel } from '../src/lib/judicial-calendar-time.mjs';
import { calendarDayDescriber } from '../src/lib/judicial-calendar-days.mjs';

// Server renders of the shared calendar components. Clicks and month changes
// are covered by the Agenda and judicial calendar browser specs.
// Hydration comments are dropped so assertions read like the DOM.
const componentMarkup = async (name, props) =>
  (await rendered(name, props)).replace(/<!--[\s\S]*?-->/g, '');
// The disabled attribute, not the disabled: variant inside class.
const isDisabled = (tag) => /\sdisabled(?=[\s>=])/.test(tag);
const buttons = (html) => html.match(/<button\b[^>]*>/g) ?? [];
const attribute = (tag, name) => tag.match(new RegExp(`\\s${name}="([^"]*)"`))?.[1];
const named = (html, label) => buttons(html).find((tag) => attribute(tag, 'aria-label') === label);
const describe = (date) => ({
  label: `${civilLabel(date)}: Computable`,
  tone: 'countable',
  marker: date === '2026-11-03' ? { dot: true } : { text: 'C' },
  attrs: { 'data-state': 'countable' },
});

test('a month grid renders one button per day of the month', async () => {
  const html = await componentMarkup('MonthGrid', {
    month: '2026-11',
    today: '2026-11-05',
    selected: '2026-11-10',
    describe,
  });
  const days = buttons(html);
  assert.equal(days.length, 30);
  assert.ok(days.every((tag) => attribute(tag, 'type') === 'button'));
  assert.ok(days.every((tag) => attribute(tag, 'data-state') === 'countable'));
  assert.equal(
    attribute(named(html, '5 de noviembre de 2026: Computable'), 'aria-current'),
    'date',
  );
  assert.equal(html.match(/aria-current="date"/g).length, 1);
  assert.equal(
    attribute(named(html, '10 de noviembre de 2026: Computable'), 'aria-pressed'),
    'true',
  );
  assert.equal(
    attribute(named(html, '11 de noviembre de 2026: Computable'), 'aria-pressed'),
    'false',
  );
  assert.equal(html.match(/aria-pressed="true"/g).length, 1);
  assert.doesNotMatch(html, /31 de octubre/);
  assert.match(html, /<time datetime="2026-11-05"[^>]*bg-brand-800/);
  assert.match(html, /<time datetime="2026-11-10"[^>]*ring-brand-800/);
});

test('a month grid names itself and keeps outside days out of the tab order', async () => {
  const html = await componentMarkup('MonthGrid', { month: '2026-11', headingLevel: 2 });
  const id = html.match(/<h2[^>]*id="([^"]+)"[^>]*>noviembre de 2026<\/h2>/)?.[1];
  assert.ok(id);
  assert.match(html, new RegExp(`aria-labelledby="${id}"`));
  assert.match(html, /<span aria-hidden="true">L<\/span><span class="sr-only">Lunes<\/span>/);
  assert.match(html, /<span[^>]*aria-hidden="true"[^>]*class="[^"]*bg-canvas[^"]*rounded-tl-lg/);
  assert.match(html, /class="[^"]*rounded-tr-lg[^"]*"[^>]*aria-label="1 de noviembre de 2026"/);
  assert.equal(buttons(html).length, 30);
  assert.ok(buttons(html).every((tag) => !isDisabled(tag)));
  const locked = await componentMarkup('MonthGrid', { month: '2026-11', disabled: true });
  assert.ok(buttons(locked).every((tag) => isDisabled(tag)));
});

test('a judicial month without loaded days disables every day', async () => {
  const html = await componentMarkup('MonthGrid', {
    month: '2026-11',
    describe: calendarDayDescriber([]),
  });
  assert.equal(buttons(html).length, 30);
  assert.ok(buttons(html).every((tag) => isDisabled(tag)));
});

test('the last civil month leaves its trailing cells empty', async () => {
  const html = await componentMarkup('MonthGrid', { month: '9999-12' });
  assert.equal(buttons(html).length, 31);
  assert.doesNotMatch(html, /10000|undefined|null/);
});

test('a month calendar shows two months with bounded navigation', async () => {
  const html = await componentMarkup('MonthCalendar', {
    month: '2026-11',
    label: 'Calendario de la agenda',
  });
  assert.match(html, /<section[^>]*aria-label="Calendario de la agenda"/);
  assert.match(html, />noviembre de 2026</);
  assert.match(html, /class="[^"]*hidden md:block[^"]*"[^>]*>[\s\S]*diciembre de 2026/);
  const controls = buttons(html).filter((tag) => tag.includes('btn-icon'));
  assert.equal(controls.length, 2);
  assert.ok(controls.every((tag) => attribute(tag, 'type') === 'button' && !isDisabled(tag)));
  assert.match(html, /<span class="sr-only">Mes anterior<\/span>/);
  assert.match(html, /<span class="sr-only">Mes siguiente<\/span>/);
  assert.match(html, /<svg/);
});

test('month calendar controls disable at their bounds', async () => {
  const icons = (html) => buttons(html).filter((tag) => tag.includes('btn-icon'));
  const covered = await componentMarkup('MonthCalendar', {
    month: '2000-02',
    months: 1,
    min: '2000-02',
    max: '2000-03',
    label: 'Fechas clasificadas',
  });
  assert.deepEqual(icons(covered).map(isDisabled), [true, false]);
  assert.doesNotMatch(covered, /marzo de 2000/);
  const last = await componentMarkup('MonthCalendar', { month: '9999-12', label: 'Fin' });
  assert.deepEqual(icons(last).map(isDisabled), [false, true]);
  assert.equal(last.match(/>diciembre de 9999</g).length, 1);
  const busy = await componentMarkup('MonthCalendar', { month: '2026-11', disabled: true });
  assert.deepEqual(icons(busy).map(isDisabled), [true, true]);
  assert.ok(buttons(busy).every(isDisabled));
});

const row = {
  key: 'hearing-7',
  value: { id: 7 },
  label: 'Consultar audiencia 7',
  attrs: { 'data-agenda-kind': 'hearing' },
  date: '2026-01-02',
  dateText: '2 de enero de 2026',
  badge: 'Audiencia',
  title: 'Audiencia inicial',
  status: 'Cancelada',
  meta: ['Caso A / EXP-1', 'Revisi\u00f3n 2'],
  time: { datetime: '2026-01-02T10:00:00-06:00', text: '10:00:00 UTC-06:00' },
  description: 'Detalle oculto',
};

test('an upcoming list renders one record button per row', async () => {
  const html = await componentMarkup('UpcomingList', {
    title: 'Actividades en el periodo',
    rows: [
      row,
      {
        ...row,
        key: 'day',
        label: 'D\u00eda',
        pressed: true,
        time: null,
        timeText: 'Todo el d\u00eda',
      },
    ],
    empty: 'Nada',
  });
  const id = html.match(/<h2[^>]*id="([^"]+)"[^>]*>Actividades en el periodo<\/h2>/)?.[1];
  assert.ok(id);
  assert.match(html, new RegExp(`<section[^>]*aria-labelledby="${id}"`));
  assert.match(html, /<ol class="[^"]*divide-y divide-line/);
  const [first, second] = buttons(html);
  assert.equal(attribute(first, 'aria-label'), 'Consultar audiencia 7');
  assert.equal(attribute(first, 'data-agenda-kind'), 'hearing');
  assert.equal(attribute(first, 'type'), 'button');
  assert.equal(attribute(first, 'aria-pressed'), undefined);
  assert.equal(attribute(second, 'aria-pressed'), 'true');
  for (const text of ['2 de enero de 2026', 'Audiencia inicial', 'Cancelada', 'Revisi\u00f3n 2'])
    assert.ok(html.includes(text), text);
  assert.match(html, /<time datetime="2026-01-02T10:00:00-06:00"[^>]*>10:00:00 UTC-06:00<\/time>/);
  assert.match(html, /Todo el d\u00eda/);
  assert.match(html, /<span class="sr-only">Detalle oculto<\/span>/);
  assert.doesNotMatch(html, />Nada</);
});

test('an empty upcoming list shows its muted line only when given one', async () => {
  const empty = await componentMarkup('UpcomingList', {
    title: 'Pr\u00f3ximos',
    level: 3,
    rows: [],
    empty: 'Sin actividades.',
  });
  assert.match(empty, /<h3[^>]*>Pr\u00f3ximos<\/h3>/);
  assert.match(empty, /<p class="[^"]*text-muted[^"]*">Sin actividades.<\/p>/);
  assert.equal(buttons(empty).length, 0);
  const quiet = await componentMarkup('UpcomingList', { title: 'X', rows: [], empty: null });
  assert.doesNotMatch(quiet, /<p\b/);
  const locked = await componentMarkup('UpcomingList', { title: 'X', rows: [row], disabled: true });
  assert.ok(isDisabled(buttons(locked)[0]));
});

test('an upcoming list bounds its date column only beside the record', async () => {
  // On phones the date sits on its own line and must not wrap at a fixed
  // width; from sm up the column is wide enough for "2 de enero de 2026".
  const html = await componentMarkup('UpcomingList', { title: 'X', rows: [row] });
  const date = html.match(/<time datetime="2026-01-02"[^>]*>/)?.[0];
  assert.ok(date);
  const classes = attribute(date, 'class').split(/\s+/);
  assert.ok(classes.includes('sm:w-40'), classes.join(' '));
  assert.ok(!classes.some((name) => /^w-/.test(name)), classes.join(' '));
});
