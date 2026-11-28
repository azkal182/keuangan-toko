param(
    [string]$Version = '0.1.0-beta.1',
    [ValidateSet('release', 'debug')]
    [string]$Profile = 'release',
    [string]$IsccPath = ''
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../../..')).Path
if (-not $IsccPath) {
    $compiler = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($compiler) { $IsccPath = $compiler.Source }
}
if (-not $IsccPath) {
    $candidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6/ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 6/ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 7/ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs/Inno Setup 7/ISCC.exe')
    )
    $IsccPath = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
}
if (-not $IsccPath -or -not (Test-Path -LiteralPath $IsccPath)) {
    throw 'Inno Setup Compiler tidak ditemukan. Pasang Inno Setup atau berikan -IsccPath.'
}

Push-Location $repoRoot
try {
    $cargoArgs = @('build', '--locked', '-p', 'toko-desktop')
    if ($Profile -eq 'release') { $cargoArgs += '--release' }
    & cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { throw "Cargo build gagal ($LASTEXITCODE)." }
    $exe = Join-Path $repoRoot "target/$Profile/toko-desktop.exe"
    if (-not (Test-Path -LiteralPath $exe)) { throw "Executable tidak ditemukan: $exe" }
    $outputVersion = if ($Profile -eq 'debug') { "$Version-debug" } else { $Version }
    & $IsccPath "/DAppVersion=$outputVersion" "/DAppExe=$exe" (Join-Path $PSScriptRoot 'piutang-reza-putra.iss')
    if ($LASTEXITCODE -ne 0) { throw "Pembuatan installer gagal ($LASTEXITCODE)." }
} finally {
    Pop-Location
}
