import type { CSSProperties } from 'react';

const paths = {
  till: 'M3 3h18v12H3z M7 19h10 M12 15v4 M7 7h4 M7 11h8',
  grid: 'M3 3h7v7H3z M14 3h7v7h-7z M3 14h7v7H3z M14 14h7v7h-7z',
  box: 'm3 7 9-4 9 4v10l-9 4-9-4V7Zm0 0 9 4 9-4 M12 11v10 M7 5l10 5',
  receipt: 'M5 3h14v18l-3-2-4 2-4-2-3 2V3Z M9 7h6 M9 11h6 M9 15h3',
  chart: 'M4 3v17h17 M8 15v-4 M13 15V6 M18 15V9',
  settings: 'M4 7h16 M4 17h16 M8 4v6 M16 14v6',
  shield: 'm12 3 8 3v6c0 5-8 9-8 9s-8-4-8-9V6l8-3Z m-4 9 3 3 5-6',
  logout: 'M9 4H4v16h5 M10 12h11 m-4-4 4 4-4 4',
  search: 'M16 10a6 6 0 1 1-12 0 6 6 0 0 1 12 0Zm-1 5 6 6',
  barcode: 'M3 7V3h4 M17 3h4v4 M21 17v4h-4 M7 21H3v-4 M7 7v10 M10 7v10 M14 7v10 M17 7v10',
  plus: 'M12 5v14 M5 12h14',
  minus: 'M5 12h14',
  arrow: 'M4 12h16 m-6-6 6 6-6 6',
  close: 'm6 6 12 12 M6 18 18 6',
  bag: 'M5 7h14l1 14H4L5 7Z M9 9V6a3 3 0 0 1 6 0v3',
  cash: 'M3 5h18v14H3z M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0Z M6 8h1 M17 16h1',
  card: 'M3 5h18v14H3z M3 10h18 M7 15h3',
  clock: 'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z M12 7v5l3 2',
  check: 'm5 12 4 4L19 6',
  store: 'M3 10 5 3h14l2 7 M3 10c0 4 5 4 5 0 0 4 8 4 8 0 0 4 5 4 5 0 M5 13v8h14v-8 M9 21v-6h6v6',
  spark: 'm12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3Z',
} as const;

export type IconName = keyof typeof paths;
export function Icon({ name, size = 20 }: { name: IconName; size?: number }) {
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false"><path d={paths[name]} /></svg>;
}

/** Stable department colours, with no network assets or changes to catalogue data. */
export function departmentStyle(id: string): CSSProperties {
  const tones = ['#ff985f', '#69d5ae', '#7aaaf7', '#c7a0ec', '#e5c471', '#6ed4d6'];
  const hash = Array.from(id).reduce((value, char) => value + char.charCodeAt(0), 0);
  return { '--item-tint': tones[hash % tones.length] } as CSSProperties;
}
