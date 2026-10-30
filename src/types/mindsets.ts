export interface Mindset {
  id: string
  name: string
  mark: string
  categories: string[]
  prompt: string
}
export interface MindsetBadge {
  id: string
  name: string
  mark: string
}
export interface MindsetCatalog {
  entries: Mindset[]
  active: MindsetBadge
  folder: string
}