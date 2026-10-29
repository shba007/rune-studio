import IconArrowClockwise from '~icons/ph/arrow-clockwise-bold'
import IconArrowLeft from '~icons/ph/arrow-left-bold'
import IconAppWindow from '~icons/ph/app-window-fill'
import IconArrowUp from '~icons/ph/arrow-up-bold'
import IconBrain from '~icons/ph/brain-fill'
import IconCaretDown from '~icons/ph/caret-down-bold'
import IconCheckCircle from '~icons/ph/check-circle-fill'
import IconEye from '~icons/ph/eye-fill'
import IconKey from '~icons/ph/key-fill'
import IconLightning from '~icons/ph/lightning-fill'
import IconCommand from '~icons/ph/command-bold'
import IconPlay from '~icons/ph/play-fill'
import IconPause from '~icons/ph/pause-fill'
import IconSkipBack from '~icons/ph/skip-back-fill'
import IconSkipForward from '~icons/ph/skip-forward-fill'
import IconSidebar from '~icons/ph/sidebar-simple-fill'
import IconArrowsOut from '~icons/ph/arrows-out-simple-bold'
import IconEyeSlash from '~icons/ph/eye-slash-fill'
import IconHistory from '~icons/ph/clock-counter-clockwise-fill'
import IconX from '~icons/ph/x-bold'
import IconPlus from '~icons/ph/plus-bold'
import IconMinus from '~icons/ph/minus-bold'
import IconArrowsIn from '~icons/ph/arrows-in-simple-bold'
import IconCopy from '~icons/ph/copy-fill'
import IconHeadphones from '~icons/ph/headphones-fill'
import IconHourglass from '~icons/ph/hourglass-medium-fill'
import IconKeyboard from '~icons/ph/keyboard-fill'
import IconLinkSimple from '~icons/ph/link-simple-fill'
import IconMicrophone from '~icons/ph/waveform-bold'
import IconMonitor from '~icons/ph/monitor-fill'
import IconMouse from '~icons/ph/mouse-simple-fill'
import IconPlugs from '~icons/ph/plugs-connected-fill'
import IconScan from '~icons/ph/scan-bold'
import IconShieldCheck from '~icons/ph/shield-check-fill'
import IconSlidersHorizontal from '~icons/ph/sliders-horizontal-bold'
import IconRuneMark from '~icons/custom/rune-mark'
import IconStopCircle from '~icons/ph/stop-fill'
import IconVideo from '~icons/ph/video-camera-fill'
import IconTrash from '~icons/ph/trash-fill'
import IconMasks from '~icons/ph/mask-happy-fill'
import IconGraph from '~icons/ph/graph-fill'
import IconTarget from '~icons/ph/target-bold'
import IconMagnifyingGlass from '~icons/ph/magnifying-glass-bold'
import IconDatabase from '~icons/ph/database-bold'
import IconInfo from '~icons/ph/info-bold'
import IconDotsThree from '~icons/ph/dots-three-bold'
import IconCheck from '~icons/ph/check-bold'

export const icons = {
  info: IconInfo,
  more: IconDotsThree,
  check: IconCheck,
  search: IconMagnifyingGlass,
  data: IconDatabase,
  personality: IconMasks,
  graph: IconGraph,
  target: IconTarget,
  app: IconAppWindow,
  back: IconArrowLeft,
  chevron: IconCaretDown,
  close: IconX,
  history: IconHistory,
  minimize: IconMinus,
  maximize: IconArrowsOut,
  restore: IconArrowsIn,
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
  add: IconPlus,
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