// Turns Agenda items into the rows of UpcomingList and the day descriptions
// of MonthCalendar. Every visible fact stays in text: the button names, the
// data-agenda-* attributes and the meta lines are what the Agenda specs read.
import { hearingKinds, hearingStatus, hearingTimeLabel } from '../lib/hearings.mjs';
import { deadlineInstantLabel } from '../lib/deadline-time.mjs';
import { agendaCivilDay } from '../lib/agenda-periods.mjs';
import { civilLabel } from '../lib/judicial-calendar-time.mjs';
import { periodContains } from '../lib/month-grid.mjs';
import { resourceHearingKinds } from '../lib/resource-hearing-values.mjs';
import { precautionaryHearingPurposes } from '../lib/precautionary-hearing-presentation.mjs';
import { agendaIdentity, agendaRecord } from './agenda-presentation.mjs';

const kindLabels = {
  hearing: 'Audiencia',
  deadline: 'Plazo',
  resource_hearing: 'Audiencia de recurso',
  precautionary_hearing: 'Audiencia cautelar',
};

function itemTitle(kind, record) {
  if (kind === 'precautionary_hearing') return precautionaryHearingPurposes[record.purpose];
  if (kind === 'resource_hearing') return resourceHearingKinds[record.kind];
  if (kind === 'hearing') return hearingKinds[record.kind]?.label || record.kind;
  return record.title;
}

// Offsets are explicit '+HH:MM' / '-HH:MM' strings already validated by the
// Agenda query.
function offsetSeconds(offset) {
  const minutes = Number(offset.slice(1, 3)) * 60 + Number(offset.slice(4));
  return (offset[0] === '-' ? -minutes : minutes) * 60;
}

// The details below the title: case, revision, projected time, the hearing
// status or deadline responsibility, and administrative closure.
function itemMeta(kind, record, context, projected) {
  const scheduled = kind !== 'deadline';
  const meta = [
    `${context.case_title} / ${context.case_reference}`,
    `Revisi\u00f3n ${record.revision}`,
    `${scheduled ? 'Hora en la agenda' : 'Vencimiento operativo'}: ${projected}`,
  ];
  if (kind === 'hearing' || kind === 'precautionary_hearing')
    meta.push(`${hearingStatus[record.status]} / ${hearingTimeLabel(record.scheduled_at)}`);
  else if (kind === 'resource_hearing')
    meta.push(`Horario declarado / ${hearingTimeLabel(record.scheduled_at)}`);
  else
    meta.push(
      `Responsable: ${record.responsible.email}`,
      record.attention_recorded ? 'Atenci\u00f3n declarada' : 'Atenci\u00f3n pendiente',
    );
  if (context.case_status === 'closed') meta.push('Expediente cerrado administrativamente');
  return meta;
}

// One UpcomingList row for an Agenda item, projected onto the queried offset.
// Items carry a single instant, so the row shows that time and no range.
export function agendaRow(item, offset) {
  const record = agendaRecord(item),
    kind = item.kind,
    badge = kindLabels[kind];
  // Ordinary hearings carry their case fields on the record; the other
  // families carry them on the item.
  const context = kind === 'hearing' ? record : item;
  const projected = deadlineInstantLabel({ ...item.at, offset_seconds: offsetSeconds(offset) });
  const [day, clock] = projected.split(' ');
  const date = agendaCivilDay(item.at, offset);
  const cancelled =
    (kind === 'hearing' || kind === 'precautionary_hearing') && record.status === 'cancelled';
  return {
    key: agendaIdentity(item),
    value: item,
    label: `Consultar ${badge.toLowerCase()} ${record.id}`,
    attrs: { 'data-agenda-kind': kind },
    date,
    dateText: civilLabel(date),
    badge,
    title: itemTitle(kind, record),
    status: cancelled ? 'Cancelada' : undefined,
    meta: itemMeta(kind, record, context, projected),
    time: {
      datetime: `${day}T${clock.slice(0, 8)}${offset}`,
      text: `${clock.replace(/\.0{9}$/, '')} UTC${offset}`,
    },
  };
}

const count = (total, complete) => {
  const noun = total === 1 ? 'actividad' : 'actividades';
  const loaded = complete ? '' : total === 1 ? ' cargada' : ' cargadas';
  return `${total} ${noun}${loaded}`;
};

// describe(date) for MonthCalendar. Days of the queried period [from, until)
// are tinted, named with how many activities they hold (or "cargadas" while
// the query is partial) and carry data-agenda-day; a dot marks the days with
// activities. Other days only say they are outside the period.
export function agendaCalendarDay({ groups, range, complete }) {
  return (date) => {
    if (!periodContains(date, range.from, range.until))
      return {
        label: `${civilLabel(date)}: fuera del periodo consultado`,
        tone: null,
        marker: null,
        attrs: {},
      };
    const total = groups.get(date)?.length ?? 0;
    const summary = total
      ? count(total, complete)
      : complete
        ? 'sin actividades'
        : 'sin actividades cargadas';
    return {
      label: `${civilLabel(date)}: ${summary}`,
      tone: 'period',
      marker: total ? { dot: true } : null,
      attrs: { 'data-agenda-day': date },
    };
  };
}
