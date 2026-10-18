/**
 * The only place an icon set is named. The UI asks for a meaning ("mic", "send") through
 * `<AppIcon>` and never imports an icon directly (a lint rule enforces this), so changing the
 * family or style is a one-file change.
 *
 * Style rule: filled (Phosphor `-fill`) everywhere. State is carried by the container and the
 * colour (a dark circle, red while listening), never by swapping between outline and filled.
 * Hand-made icons live in src/assets/icons and are registered here as `~icons/custom/<name>`.
 */
import IconArrowClockwise from '~icons/ph/arrow-clockwise-fill'
import IconArrowLeft from '~icons/ph/arrow-left-fill'
import IconAppWindow from '~icons/ph/app-window-fill'
import IconArrowUp from '~icons/ph/arrow-up-fill'
import IconCheckCircle from '~icons/ph/check-circle-fill'
import IconEye from '~icons/ph/eye-fill'
import IconKey from '~icons/ph/key-fill'
import IconCommand from '~icons/ph/command-fill'
import IconCopy from '~icons/ph/copy-fill'
import IconCursorClick from '~icons/ph/cursor-click-fill'
import IconHourglass from '~icons/ph/hourglass-medium-fill'
import IconKeyboard from '~icons/ph/keyboard-fill'
import IconLinkSimple from '~icons/ph/link-simple-fill'
import IconMicrophone from '~icons/ph/microphone-fill'
import IconPlugs from '~icons/ph/plugs-connected-fill'
import IconSlidersHorizontal from '~icons/ph/sliders-horizontal-fill'
import IconRuneMark from '~icons/custom/rune-mark'
import IconStopCircle from '~icons/ph/stop-circle-fill'
import IconTrash from '~icons/ph/trash-fill'

export const icons = {
  app: IconAppWindow,
  back: IconArrowLeft,
  click: IconCursorClick,
  copy: IconCopy,
  plug: IconPlugs,
  keyboard: IconKeyboard,
  wait: IconHourglass,
  eye: IconEye,
  key: IconKey,
  link: IconLinkSimple,
  logo: IconRuneMark,
  mic: IconMicrophone,
  ok: IconCheckCircle,
  refresh: IconArrowClockwise,
  send: IconArrowUp,
  shortcut: IconCommand,
  stop: IconStopCircle,
  trash: IconTrash,
  tune: IconSlidersHorizontal,
} as const

export type IconName = keyof typeof icons