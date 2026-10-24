// changelogen is the only commit parser, bump calculator and Markdown renderer.
export const releasePolicy = {
  preOneFeatures: 'minor',
  preOneBreaking: 'minor',
  stableFeatures: 'minor',
  stableBreaking: 'major',
  maintenance: 'patch',
  // Existing Git signatures and historical contributor links identify this account.
  // Keep attribution deterministic without exposing email addresses in new notes.
  authorHandles: {
    'shirsendu2001@gmail.com': 'shba007',
    '48382105+shba007@users.noreply.github.com': 'shba007',
  },
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