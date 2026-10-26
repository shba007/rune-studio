export interface Personality {
  id: string
  name: string
  mark: string
  categories: string[]
  prompt: string
}
export interface PersonalityBadge {
  id: string
  name: string
  mark: string
}
export interface PersonalityCatalog {
  entries: Personality[]
  active: PersonalityBadge
  folder: string
}