$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$scratch = Join-Path $PSScriptRoot 'target/migration-check'
New-Item -ItemType Directory -Force -Path (Join-Path $scratch 'src') | Out-Null
Copy-Item -Path (Join-Path $repo 'apps/desktop/src/*') -Destination (Join-Path $scratch 'src') -Recurse -Force

$manifest = @'
[package]
name = "desktop-migration-check"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
gpui = { package = "gpui-pre", version = "=0.3.14" }
gpui-kit = "=0.11.0"
chrono = "0.4"
serde_json = "1"
ureq = { version = "2", features = ["cookies", "json"] }
uuid = { version = "1", features = ["v4"] }
'@
[IO.File]::WriteAllText((Join-Path $scratch 'Cargo.toml'), $manifest)

$appPath = Join-Path $scratch 'src/app/mod.rs'
$app = [IO.File]::ReadAllText($appPath)
if ($app.Contains('Application::new().run(')) {
    $app = $app.Replace('Application::new().run(', 'gpui_kit::application().run(')
    $app = $app.Replace('App, AppContext, Application,', 'App, AppContext,')
} elseif (-not $app.Contains('gpui_kit::application()')) {
    throw 'Desktop startup changed; review the migration probe.'
}
[IO.File]::WriteAllText($appPath, $app)

cargo check --offline --manifest-path (Join-Path $scratch 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
