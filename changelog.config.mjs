// changelogen is the only commit parser, bump calculator and Markdown renderer.
export const releasePolicy = {
  preOneFeatures: 'minor',
  preOneBreaking: 'minor',
  stableFeatures: 'minor',
  stableBreaking: 'major',
  maintenance: 'patch',
  authorHandles: {}, // Optional reviewed email -> GitHub handle map; no online lookup.
}
export default {
  repo: { provider: 'github', domain: 'github.com', repo: 'shba007/rune-studio' },
  templates: { tagBody: 'v{{newVersion}}' },
  hideAuthorEmail: true,
  noAuthors: false,
  types: {
    feat: { title: '🚀 Enhancements', semver: 'minor' },
    fix: { title: '🩹 Fixes', semver: 'patch' },
    perf: { title: '🔥 Performance', semver: 'patch' },
    docs: { title: '📖 Documentation', semver: 'patch' },
    refactor: { title: '💅 Refactors', semver: 'patch' },
    style: { title: '🎨 Styles', semver: 'patch' },
    test: { title: '✅ Tests', semver: 'patch' },
    build: { title: '📦 Build', semver: 'patch' },
    ci: { title: '🤖 CI', semver: 'patch' },
    chore: { title: '🏡 Chore', semver: 'patch' },
    revert: { title: '↩️ Reverts', semver: 'patch' },
  },
}