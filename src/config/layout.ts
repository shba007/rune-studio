export const ORB_SIZE = 64
export const PAGE_GAP = 8

export const DOCK_WIDTH = 32
export const DOCK_GAP = 0

export const PAGE_WIDTH = 272

export function pillContentHeight(cards: number, composer: number): number {
  return cards + composer + (composer > 0 ? PAGE_GAP : 0)
}