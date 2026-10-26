import type { IconName } from '@/icons'

export const COMPOSER_COMMANDS = [
  {
    id: 'analysis',
    command: '/visual analysis',
    description: 'Choose a screen or window to analyze',
    icon: 'scan',
  },
  { id: 'goals', command: '/goals', description: 'Open Goals in Knowledge', icon: 'thinking' },
  {
    id: 'tasks',
    command: '/tasks',
    description: 'Open task history and recordings',
    icon: 'history',
  },
  { id: 'settings', command: '/settings', description: 'Open Rune settings', icon: 'tune' },
] as const satisfies readonly { id: string; command: string; description: string; icon: IconName }[]

export type ComposerCommand = (typeof COMPOSER_COMMANDS)[number]
const normalized = (text: string) =>
  text
    .trim()
    .toLowerCase()
    .replace(/[-_\s]+/g, ' ')
export function composerCommands(text: string): ComposerCommand[] {
  if (!text.trimStart().startsWith('/')) return []
  const query = normalized(text)
  return COMPOSER_COMMANDS.filter((item) => item.command.startsWith(query))
}
export function exactComposerCommand(text: string): ComposerCommand | undefined {
  return COMPOSER_COMMANDS.find((item) => item.command === normalized(text))
}