/**
 * Geometry shared by every page in the pill. src-tauri/src/window.rs (`Mode::size`) mirrors
 * these numbers: the window is DOCK_WIDTH + DOCK_GAP + ORB_SIZE + PAGE_GAP + PAGE_WIDTH wide while a
 * page is open. Change them together.
 */
export const ORB_SIZE = 64
export const PAGE_GAP = 8

/** The capsule with Settings and Analyse the screen, to the left of the orb (only while open). */
export const DOCK_WIDTH = 32
export const DOCK_GAP = 0

/** Every page that sits beside the orb (composer, settings, future task views) is this wide. */
export const PAGE_WIDTH = 272