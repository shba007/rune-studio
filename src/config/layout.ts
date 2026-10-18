/**
 * Geometry shared by every page in the pill. src-tauri/src/window.rs (`Mode::size`) mirrors
 * these numbers: the window is ORB_SIZE + PAGE_GAP + PAGE_WIDTH wide while a page is open.
 * Change them together.
 */
export const ORB_SIZE = 64
export const PAGE_GAP = 8

/** Every page that sits beside the orb (composer, settings, future task views) is this wide. */
export const PAGE_WIDTH = 272