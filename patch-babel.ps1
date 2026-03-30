$ErrorActionPreference = "Stop"
$file = 'C:\Users\likwi\OneDrive\Desktop\PROJS\SimSort\node_modules\.pnpm\@babel+helper-compilation-targets@7.28.6\node_modules\@babel\helper-compilation-targets\lib\index.js'
$content = Get-Content -Raw $file -Encoding UTF8

$old = 'var _lruCache = require("lru-cache");'
$new = 'var _lruCache = require("lru-cache");
if (typeof _lruCache !== "function") { _lruCache = _lruCache.default || Object.values(_lruCache)[0]; }'

if ($content -notmatch [regex]::Escape($old)) {
    Write-Output "Pattern not found! Current require line:"
    $content -split "`n" | Select-String "lruCache" | Select-Object -First 3
    exit 1
}

$newContent = $content -replace [regex]::Escape($old), $new
Set-Content -Path $file -Value $newContent -Encoding UTF8 -NoNewline
Write-Output "Patched successfully"
