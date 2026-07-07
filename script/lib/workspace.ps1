
function ParseSimWorkspace {
    $metadata = cargo metadata --no-deps --offline | ConvertFrom-Json
    $env:SIM_WORKSPACE = $metadata.workspace_root
    $env:RELEASE_VERSION = $metadata.packages | Where-Object { $_.name -eq "sim" } | Select-Object -ExpandProperty version
}
