import { readFileSync, writeFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

export function releaseNotes(changelog, version) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Invalid release version')
  const sections = [...changelog.matchAll(/^## (?:v)?(\d+\.\d+\.\d+)[ \t]*\r?$/gm)]
  const matches = sections.filter((entry) => entry[1] === version)
  if (matches.length !== 1 || sections[0]?.[1] !== version)
    throw new Error(`The first CHANGELOG version section must uniquely match ${version}`)
  const start = matches[0].index + matches[0][0].length
  const notes = changelog.slice(start, sections[1]?.index ?? changelog.length).trim()
  const substantive = notes
    .split('\n')
    .filter((line) => line.trim() && !/^\s*(#|\[compare changes\])/.test(line))
    .join('\n')
  if (
    !substantive ||
    /\b(TODO|TBD|placeholder|unreleased|cycle\s*\d+|milestone\s*\w+|ci\(release\):|update version|bump version)\b/i.test(
      notes,
    )
  )
    throw new Error(
      `CHANGELOG ${version} requires substantive public notes without placeholders or internal planning identifiers`,
    )
  return `${notes}\n`
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const version = JSON.parse(readFileSync('package.json', 'utf8')).version
    const notes = releaseNotes(readFileSync('CHANGELOG.md', 'utf8'), version)
    if (process.argv[2] === '--out' && process.argv[3]) writeFileSync(process.argv[3], notes)
    else if (process.argv.length === 2) process.stdout.write(notes)
    else throw new Error('Usage: node scripts/release-notes.mjs [--out FILE]')
  } catch (error) {
    console.error(`Release preparation failed: ${error.message}`)
    process.exitCode = 1
  }
}