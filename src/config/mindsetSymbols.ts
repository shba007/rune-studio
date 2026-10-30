export const mindsetSymbols = [
  {
    id: 'companion',
    label: 'Spark · Companion',
    path: 'M12 3 14.5 9.5 21 12 14.5 14.5 12 21 9.5 14.5 3 12 9.5 9.5Z',
  },
  {
    id: 'fitness',
    label: 'Dumbbell · Fitness',
    path: 'M7 8v8M4 9v6M17 8v8M20 9v6M7 12h10M2 12h2M20 12h2',
  },
  { id: 'finance', label: 'Chart · Finance', path: 'M4 4v16h16M8 16v-4M12 16V8M16 16V5' },
  {
    id: 'learning',
    label: 'Book · Learning',
    path: 'M12 6v15M12 6C9 3 5 3 2 4v14c4-1 7 0 10 3 3-3 6-4 10-3V4c-3-1-7-1-10 2Z',
  },
  {
    id: 'creativity',
    label: 'Wand · Creativity',
    path: 'm4 20 12-12 3 3L7 23ZM13 3v3M19 2v3M21 7h3M4 5v4M2 7h4',
  },
  {
    id: 'focus',
    label: 'Target · Focus',
    path: 'M21 12a9 9 0 1 1-9-9M17 12a5 5 0 1 1-5-5M12 12l9-9M17 3h4v4',
  },
] as const
export function mindsetSymbol(mark: string) {
  return mindsetSymbols.find((symbol) => symbol.id === mark) ?? mindsetSymbols[0]
}