
function ParseBaymaxWorkspace {
    $metadata = cargo metadata --no-deps --offline | ConvertFrom-Json
    $env:BAYMAX_WORKSPACE = $metadata.workspace_root
    $env:RELEASE_VERSION = $metadata.packages | Where-Object { $_.name -eq "baymax" } | Select-Object -ExpandProperty version
}
