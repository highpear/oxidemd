$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$requiredVersion = "cargo-about 0.9.1"
$cargoAbout = Get-Command cargo-about -ErrorAction SilentlyContinue

if (!$cargoAbout) {
    throw "cargo-about 0.9.1 is required: cargo install --locked --features cli --version 0.9.1 cargo-about"
}

$installedVersion = cargo about --version
if ($LASTEXITCODE -ne 0 -or $installedVersion -ne $requiredVersion) {
    throw "Expected $requiredVersion, found $installedVersion"
}

Push-Location $repoRoot
try {
    cargo about generate `
        --locked `
        --fail `
        --output-file THIRD_PARTY_LICENSES.html `
        about.hbs

    if ($LASTEXITCODE -ne 0) {
        throw "cargo-about failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}
