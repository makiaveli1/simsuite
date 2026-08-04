$ErrorActionPreference = 'Stop'

throw @'
This legacy node_modules patch is retired.

SimSuite now resolves the Babel/lru-cache compatibility issue through the pinned pnpm override in package.json. Do not modify installed dependency files directly.

Run this from the repository root instead:
  pnpm install --frozen-lockfile
'@
