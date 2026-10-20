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
import IconBrain from '~icons/ph/brain-fill'
import IconCaretDown from '~icons/ph/caret-down-fill'
import IconCheckCircle from '~icons/ph/check-circle-fill'
import IconEye from '~icons/ph/eye-fill'
import IconKey from '~icons/ph/key-fill'
import IconLightning from '~icons/ph/lightning-fill'
import IconCommand from '~icons/ph/command-fill'
import IconPlay from '~icons/ph/play-fill'
import IconPause from '~icons/ph/pause-fill'
import IconSkipBack from '~icons/ph/skip-back-fill'
import IconSkipForward from '~icons/ph/skip-forward-fill'
import IconSidebar from '~icons/ph/sidebar-simple-fill'
import IconArrowsOut from '~icons/ph/arrows-out-simple-fill'
import IconEyeSlash from '~icons/ph/eye-slash-fill'
import IconX from '~icons/ph/x-fill'
import IconCopy from '~icons/ph/copy-fill'
import IconHeadphones from '~icons/ph/headphones-fill'
import IconHourglass from '~icons/ph/hourglass-medium-fill'
import IconKeyboard from '~icons/ph/keyboard-fill'
import IconLinkSimple from '~icons/ph/link-simple-fill'
import IconMicrophone from '~icons/ph/microphone-fill'
import IconMonitor from '~icons/ph/monitor-fill'
import IconMouse from '~icons/ph/mouse-simple-fill'
import IconPlugs from '~icons/ph/plugs-connected-fill'
import IconScan from '~icons/ph/scan-fill'
import IconShieldCheck from '~icons/ph/shield-check-fill'
import IconSlidersHorizontal from '~icons/ph/sliders-horizontal-fill'
import IconRuneMark from '~icons/custom/rune-mark'
import IconStopCircle from '~icons/ph/stop-circle-fill'
import IconVideo from '~icons/ph/video-camera-fill'
import IconTrash from '~icons/ph/trash-fill'

export const icons = {
  app: IconAppWindow,
  back: IconArrowLeft,
  chevron: IconCaretDown,
  close: IconX,
  copy: IconCopy,
  plug: IconPlugs,
  keyboard: IconKeyboard,
  wait: IconHourglass,
  eye: IconEye,
  headphones: IconHeadphones,
  key: IconKey,
  link: IconLinkSimple,
  logo: IconRuneMark,
  mic: IconMicrophone,
  monitor: IconMonitor,
  mouse: IconMouse,
  ok: IconCheckCircle,
  refresh: IconArrowClockwise,
  scan: IconScan,
  send: IconArrowUp,
  shield: IconShieldCheck,
  shortcut: IconCommand,
  stop: IconStopCircle,
  trash: IconTrash,
  thinking: IconBrain,
  fast: IconLightning,
  tune: IconSlidersHorizontal,
  video: IconVideo,
  play: IconPlay,
  pause: IconPause,
  prev: IconSkipBack,
  next: IconSkipForward,
  drawer: IconSidebar,
  expand: IconArrowsOut,
  hide: IconEyeSlash,
} as const

export type IconName = keyof typeof icons