// Decides which section tabs stay in the row and which move to the overflow
// menu, so the row never scrolls sideways. `widths` holds the measured width
// of each tab in order, `gap` the space between two tabs, `more` the width of
// the overflow button and `active` the index of the current tab (-1 for none).
// The current tab always stays in the row: when it would not fit in its own
// position it takes the place of the last tabs that did.
export function fitTabs(widths, available, gap, more, active) {
  const all = widths.map((_, index) => index);
  const total = widths.reduce((sum, width) => sum + width, 0) + gap * Math.max(0, all.length - 1);
  if (total <= available) return { visible: all, overflow: [] };
  const room = available - more - gap;
  const visible = [];
  let used = 0;
  for (const index of all) {
    const next = used + (visible.length ? gap : 0) + widths[index];
    if (next > room) break;
    visible.push(index);
    used = next;
  }
  if (active >= 0 && !visible.includes(active)) {
    while (visible.length && used + gap + widths[active] > room) {
      const last = visible.pop();
      used -= widths[last] + (visible.length ? gap : 0);
    }
    visible.push(active);
  }
  return { visible, overflow: all.filter((index) => !visible.includes(index)) };
}
