// gen-about.mjs: rebuilds about.json (the per-app `about` blocks publish.sh puts in manifest.json)
// from the rng.media app pages' copy. Run: node scripts/gen-about.mjs [~/rng-ai-suite]
import { readFileSync, writeFileSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { homedir } from 'node:os'

const site = process.argv[2] || join(homedir(), 'rng-ai-suite')
const here = dirname(fileURLToPath(import.meta.url))
const tmp = join(here, '..', '.apps-copy.mts')
writeFileSync(tmp, readFileSync(join(site, 'lib/apps.ts'), 'utf8').replace(/^import type .*$/m, ''))
const { APPS, LICENCES } = await import(tmp)
const out = {}
for (const [id, a] of Object.entries(APPS)) {
  const c = a.copy.en
  out[id] = {
    pitch: c.pitch,
    intro: c.intro,
    description: c.what,
    features: c.features.map(f => ({ title: f.title, body: f.body })),
    screenshots: c.shots.map(s => ({ url: `https://www.rng.media${s.src}`, width: s.width, height: s.height, title: s.title, caption: s.body })),
    page: `https://www.rng.media/${id}`,
    credit: a.upstream
      ? `${a.name} is based on ${a.upstream} by the ArtCraft team, licensed MIT OR Apache-2.0, and modified by RNG Media S.L. RNG Media is not affiliated with or endorsed by the ArtCraft team.`
      : `${a.name} is made by RNG Media S.L. It uses FFmpeg under the LGPL and other open-source components listed in its licences.`,
    licences: `${LICENCES}/${id}`,
  }
}
writeFileSync(join(here, '..', 'about.json'), JSON.stringify(out, null, 2) + '\n')
;(await import('node:fs')).unlinkSync(tmp)
console.log('about.json:', Object.keys(out).join(', '))
