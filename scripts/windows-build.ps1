[CmdletBinding()]
Param(
    [Parameter()][string]$Architecture,
    [Parameter()][switch]$Install,
    [Parameter()][switch]$Help,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$ExtraArgs
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$repoRoot = Resolve-Path "$PSScriptRoot/.."
$bundleScript = Join-Path $repoRoot "script/bundle-windows.ps1"

if ($Help) {
    Write-Output "Usage: pwsh scripts/windows-build.ps1 [-Architecture x86_64|aarch64] [-Install] [-- extra bundle args]"
    Write-Output "Wraps script/bundle-windows.ps1 for the Goose migration development scripts."
    exit 0
}

$arguments = @()
if ($Architecture) {
    $arguments += @("-Architecture", $Architecture)
}
if ($Install) {
    $arguments += "-Install"
}
if ($ExtraArgs) {
    $arguments += $ExtraArgs
}

& $bundleScript @arguments
