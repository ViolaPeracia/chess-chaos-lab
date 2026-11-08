param([Parameter(Mandatory = $true)][string]$Version)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw 'Version must be X.Y.Z' }
$project = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..')).Path
$dist = Join-Path $project 'dist'
$name = "chess-chaos-lab-v$Version-windows-x64"
$stage = Join-Path $dist $name
$zip = Join-Path $dist "$name.zip"

Push-Location $project
try {
    $metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed' }
    $manifestVersion = ($metadata.packages | Where-Object name -eq 'chess-chaos-lab').version
    if ($manifestVersion -ne $Version) { throw "Tag version $Version does not match app version $manifestVersion" }
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }

    New-Item -ItemType Directory -Path $dist -Force | Out-Null
    $distPath = [IO.Path]::GetFullPath($dist).TrimEnd('\', '/')
    $stagePath = [IO.Path]::GetFullPath($stage)
    if (-not $stagePath.StartsWith("$distPath$([IO.Path]::DirectorySeparatorChar)", [StringComparison]::OrdinalIgnoreCase)) {
        throw 'Package staging path is outside dist'
    }
    if (Test-Path -LiteralPath $stagePath) { Remove-Item -LiteralPath $stagePath -Recurse -Force }
    New-Item -ItemType Directory -Path (Join-Path $stage 'docs'), (Join-Path $stage 'source') -Force | Out-Null
    Copy-Item -LiteralPath 'target/release/chess-chaos-lab.exe', 'README.md', 'LICENSE', 'CHANGELOG.md' -Destination $stage
    Copy-Item -LiteralPath 'docs/user-guide.md', 'docs/known-limitations.md' -Destination (Join-Path $stage 'docs')
    Copy-Item -LiteralPath 'Cargo.toml', 'Cargo.lock' -Destination (Join-Path $stage 'source')
    Copy-Item -LiteralPath 'crates', 'apps' -Destination (Join-Path $stage 'source') -Recurse
    Set-Content -LiteralPath (Join-Path $stage 'VERSION.txt') -Value $Version
    Compress-Archive -Path (Join-Path $stage '*') -DestinationPath $zip -Force
    $digest = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath (Join-Path $dist 'SHA256SUMS.txt') -Value "$digest  $name.zip"
    Write-Output "Package: $zip"
    Write-Output "SHA-256: $digest"
}
finally { Pop-Location }
