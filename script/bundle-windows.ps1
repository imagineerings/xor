[CmdletBinding()]
Param(
    [Parameter()][Alias('i')][switch]$Install,
    [Parameter()][Alias('h')][switch]$Help,
    [Parameter()][Alias('a')][string]$Architecture,
    [Parameter()][string]$Name,
    [Parameter()][switch]$Comfy,
    [Parameter()][switch]$RustTools,
    [Parameter()][switch]$DryRun
)

if ($DryRun) {
    $rustToolFeatures = if ($RustTools) { "rust-tools" } else { "none" }
    if ($Comfy) {
        $simFeatures = if ($RustTools) { "comfy,rocm,directml,rust-tools" } else { "comfy,rocm,directml" }
        Write-Output "mode=comfy packages=zed,cli,comfy_worker,auto_update_helper zed_features=$simFeatures remote_features=$rustToolFeatures worker_features=rocm,directml include_comfy_worker=true rust_tools=$($RustTools.ToString().ToLower())"
    }
    else {
        Write-Output "mode=default packages=zed,cli,auto_update_helper zed_features=$rustToolFeatures remote_features=$rustToolFeatures include_comfy_worker=false rust_tools=$($RustTools.ToString().ToLower())"
    }
    exit 0
}

. "$PSScriptRoot/lib/workspace.ps1"

# https://stackoverflow.com/questions/57949031/powershell-script-stops-if-program-fails-like-bash-set-o-errexit
$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $true

$buildSuccess = $false
$canCodeSign = $false

$requiredProductVariables = @(
    'ZED_PRODUCT_ID', 'ZED_PRODUCT_DISPLAY_NAME', 'ZED_PRODUCT_EXECUTABLE',
    'ZED_PRODUCT_BUNDLE_ID', 'ZED_PRODUCT_URL_SCHEME', 'ZED_PRODUCT_DATA_NAMESPACE', 'ZED_PRODUCT_ICON_SET',
    'ZED_PRODUCT_APP_FEATURES', 'ZED_PRODUCT_REMOTE_FEATURES',
    'ZED_PRODUCT_WINDOWS_INSTALLER_ID', 'ZED_PRODUCT_ARTIFACT_NAME'
)
foreach ($variableName in $requiredProductVariables) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($variableName))) {
        throw "Missing resolved product variable: $variableName. Run cargo xtask bundle --product <id>."
    }
}

$OSArchitecture = switch ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture) {
    "X64" { "x86_64" }
    "Arm64" { "aarch64" }
    default { throw "Unsupported architecture" }
}

$Architecture = if ($Architecture) {
    $Architecture
} else {
    $OSArchitecture
}

$CargoTargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { "./target" }
$CargoOutDir = "$CargoTargetDir/$Architecture-pc-windows-msvc/release"

function Get-VSArch {
    param(
        [string]$Arch
    )

    switch ($Arch) {
        "x86_64" { "amd64" }
        "aarch64" { "arm64" }
    }
}

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (-not (Test-Path $vswhere -PathType Leaf)) {
    throw "Visual Studio Installer discovery tool was not found at $vswhere"
}
$visualStudioPath = & $vswhere -latest -products * -property installationPath
$visualStudioShell = Join-Path $visualStudioPath "Common7\Tools\Launch-VsDevShell.ps1"
if (-not (Test-Path $visualStudioShell -PathType Leaf)) {
    throw "Visual Studio developer shell was not found at $visualStudioShell"
}
Push-Location
& $visualStudioShell -Arch (Get-VSArch -Arch $Architecture) -HostArch (Get-VSArch -Arch $OSArchitecture)
Pop-Location

$target = "$Architecture-pc-windows-msvc"

if ($Help) {
    Write-Output "Usage: test.ps1 [-Install] [-Help]"
    Write-Output "Build the installer for Windows.\n"
    Write-Output "Options:"
    Write-Output "  -Architecture, -a Which architecture to build (x86_64 or aarch64)"
    Write-Output "  -Install, -i      Run the installer after building."
    Write-Output "  -Comfy             Include Comfy, accelerator backends, worker, and assets."
    Write-Output "  -RustTools         Include the Cargo tool window and matching remote-server support."
    Write-Output "  -DryRun            Print the selected package plan without building it."
    Write-Output "  -Help, -h         Show this help message."
    exit 0
}

$channel = if ($env:ZED_RELEASE_CHANNEL) {
    $env:ZED_RELEASE_CHANNEL
} else {
    Get-Content "crates/zed/RELEASE_CHANNEL"
}
$env:ZED_RELEASE_CHANNEL = $channel
$env:RELEASE_CHANNEL = $channel

function CheckEnvironmentVariables {
    if($env:CI) {
        $requiredVars = @('ZED_WORKSPACE', 'RELEASE_VERSION', 'ZED_RELEASE_CHANNEL')

        foreach ($var in $requiredVars) {
            if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($var))) {
                Write-Error "$var is not set"
                exit 1
            }
        }
    }

    if ($env:ZED_DISABLE_SIGNING) {
        Write-Host "Code signing disabled by the product bundle plan"
        return
    }

    # On PRs from forks the signing secrets are not populated,
    # so skip code signing instead of failing, like bundle-mac does.
    $signingVars = @(
        'AZURE_TENANT_ID', 'AZURE_CLIENT_ID', 'AZURE_CLIENT_SECRET',
        'ACCOUNT_NAME', 'CERT_PROFILE_NAME', 'ENDPOINT',
        'FILE_DIGEST', 'TIMESTAMP_DIGEST', 'TIMESTAMP_SERVER'
    )

    $missingVars = @($signingVars | Where-Object { [string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($_)) })
    if ($missingVars.Count -eq 0) {
        $script:canCodeSign = $true
    } else {
        Write-Host "====== WARNING ======"
        Write-Host "One or more of the following variables are missing: $($missingVars -join ', ')"
        Write-Host "This bundle will not be code signed"
        Write-Host "====== WARNING ======"
    }
}

function PrepareForBundle {
    if (Test-Path "$innoDir") {
        Remove-Item -Path "$innoDir" -Recurse -Force
    }
    New-Item -Path "$innoDir" -ItemType Directory -Force
    Copy-Item -Path "$env:ZED_WORKSPACE\crates\zed\resources\windows\*" -Destination "$innoDir" -Recurse -Force
    Copy-Item -Path "$env:ZED_PRODUCT_ICON_SET\app-icon.ico" -Destination "$innoDir\product-app-icon.ico" -Force
    New-Item -Path "$innoDir\make_appx" -ItemType Directory -Force
    New-Item -Path "$innoDir\appx" -ItemType Directory -Force
    New-Item -Path "$innoDir\bin" -ItemType Directory -Force
    New-Item -Path "$innoDir\tools" -ItemType Directory -Force

    rustup target add $target
}

function GenerateLicenses {
    . $PSScriptRoot/generate-licenses.ps1
}

function BuildProductBinaries {
    Write-Output "Building product binaries for channel: $channel"
    if ($Comfy) {
        $features = "zed/comfy,zed/rocm,comfy_worker/rocm,zed/directml,comfy_worker/directml"
        if ($RustTools) {
            $features = "$features,zed/rust-tools"
        }
        cargo build --release --package zed --package cli --package comfy_worker --package auto_update_helper --features $features --target $target
    }
    else {
        $applicationFeatures = (($env:ZED_PRODUCT_APP_FEATURES -split ',') | ForEach-Object { "zed/$_" }) -join ','
        cargo build --release --package zed --package cli --package auto_update_helper --no-default-features --features $applicationFeatures --target $target
    }
    Copy-Item -Path "$CargoOutDir\zed.exe" -Destination "$innoDir\$env:ZED_PRODUCT_EXECUTABLE.exe" -Force
    Copy-Item -Path "$CargoOutDir\cli.exe" -Destination "$innoDir\cli.exe" -Force
    if ($Comfy) {
        Copy-Item -Path ".\$CargoOutDir\comfy-worker.exe" -Destination "$innoDir\comfy-worker.exe" -Force
    }
    Copy-Item -Path "$CargoOutDir\auto_update_helper.exe" -Destination "$innoDir\auto_update_helper.exe" -Force
}

function BuildRemoteServer {
    Write-Output "Building remote_server for $target"
    if (-not [string]::IsNullOrWhiteSpace($env:ZED_PRODUCT_REMOTE_FEATURES)) {
        cargo build --release --package remote_server --no-default-features --features $env:ZED_PRODUCT_REMOTE_FEATURES --target $target
    }
    else {
        cargo build --release --package remote_server --no-default-features --target $target
    }

    # Create zipped remote server binary
    $remoteServerSrc = (Resolve-Path "$CargoOutDir\remote_server.exe").Path

    if ($canCodeSign) {
        Write-Output "Code signing remote_server.exe"
        & "$innoDir\sign.ps1" $remoteServerSrc
    }

    $remoteServerDst = "$CargoTargetDir\$env:ZED_PRODUCT_ID-remote-server-windows-$Architecture.zip"
    Write-Output "Compressing remote_server to $remoteServerDst"
    Compress-Archive -Path $remoteServerSrc -DestinationPath $remoteServerDst -Force

    Write-Output "Remote server compressed successfully"
}

function ZipProductDebugSymbols {
    $items = @(
        "$CargoOutDir\zed.pdb",
        "$CargoOutDir\cli.pdb",
        "$CargoOutDir\auto_update_helper.pdb",
        "$CargoOutDir\remote_server.pdb"
    )
    if ($Comfy) {
        $items += ".\$CargoOutDir\comfy-worker.pdb"
    }

    Compress-Archive -Path $items -DestinationPath $debugArchive -Force
}


function SignZedAndItsFriends {
    if (-not $canCodeSign) {
        return
    }

    $files = "$innoDir\$env:ZED_PRODUCT_EXECUTABLE.exe,$innoDir\cli.exe,$innoDir\auto_update_helper.exe"
    if ($Comfy) {
        $files += ",$innoDir\comfy-worker.exe"
    }
    & "$innoDir\sign.ps1" $files
}

function DownloadAMDGpuServices {
    # If you update the AGS SDK version, please also update the version in `crates/gpui/src/platform/windows/directx_renderer.rs`
    $url = "https://codeload.github.com/GPUOpen-LibrariesAndSDKs/AGS_SDK/zip/refs/tags/v6.3.0"
    $zipPath = ".\AGS_SDK_v6.3.0.zip"
    # Download the AGS SDK zip file
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    # Extract the AGS SDK zip file
    Expand-Archive -Path $zipPath -DestinationPath "." -Force
}

function DownloadConpty {
    $url = "https://github.com/microsoft/terminal/releases/download/v1.23.13503.0/Microsoft.Windows.Console.ConPTY.1.23.251216003.nupkg"
    $zipPath = ".\Microsoft.Windows.Console.ConPTY.1.23.251216003.nupkg"
    Invoke-WebRequest -Uri $url -OutFile $zipPath
    Expand-Archive -Path $zipPath -DestinationPath ".\conpty" -Force
}

function CollectFiles {
    Move-Item -Path "$innoDir\cli.exe" -Destination "$innoDir\bin\$env:ZED_PRODUCT_EXECUTABLE.exe" -Force
    Move-Item -Path "$innoDir\zed.sh" -Destination "$innoDir\bin\$env:ZED_PRODUCT_EXECUTABLE" -Force
    Move-Item -Path "$innoDir\auto_update_helper.exe" -Destination "$innoDir\tools\auto_update_helper.exe" -Force
    if($Architecture -eq "aarch64") {
        New-Item -Type Directory -Path "$innoDir\arm64" -Force
        Move-Item -Path ".\conpty\build\native\runtimes\arm64\OpenConsole.exe" -Destination "$innoDir\arm64\OpenConsole.exe" -Force
        Move-Item -Path ".\conpty\runtimes\win-arm64\native\conpty.dll" -Destination "$innoDir\conpty.dll" -Force
    }
    else {
        New-Item -Type Directory -Path "$innoDir\x64" -Force
        New-Item -Type Directory -Path "$innoDir\arm64" -Force
        Move-Item -Path ".\AGS_SDK-6.3.0\ags_lib\lib\amd_ags_x64.dll" -Destination "$innoDir\amd_ags_x64.dll" -Force
        Move-Item -Path ".\conpty\build\native\runtimes\x64\OpenConsole.exe" -Destination "$innoDir\x64\OpenConsole.exe" -Force
        Move-Item -Path ".\conpty\build\native\runtimes\arm64\OpenConsole.exe" -Destination "$innoDir\arm64\OpenConsole.exe" -Force
        Move-Item -Path ".\conpty\runtimes\win-x64\native\conpty.dll" -Destination "$innoDir\conpty.dll" -Force
    }
}

function BuildInstaller {
    $issFilePath = "$innoDir\zed.iss"
    if (@('stable', 'preview', 'nightly', 'dev') -notcontains $channel) {
        throw "Can't bundle installer for unsupported channel $channel."
    }
    $appId = $env:ZED_PRODUCT_WINDOWS_INSTALLER_ID
    $appIconName = "product-app-icon"
    $appName = $env:ZED_PRODUCT_DISPLAY_NAME
    $appDisplayName = $appName
    $appSetupName = [System.IO.Path]::GetFileNameWithoutExtension($env:ZED_PRODUCT_ARTIFACT_NAME)
    $appMutex = "$env:ZED_PRODUCT_ID-$channel-Instance-Mutex"
    $appExeName = $env:ZED_PRODUCT_EXECUTABLE
    $regValueName = $env:ZED_PRODUCT_DATA_NAMESPACE
    $appUserId = $env:ZED_PRODUCT_BUNDLE_ID
    $appShellNameShort = "&$env:ZED_PRODUCT_DISPLAY_NAME"

    # Windows runner 2022 default has iscc in PATH, https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md
    # Currently, we are using Windows 2022 runner.
    # Windows runner 2025 doesn't have iscc in PATH for now, https://github.com/actions/runner-images/issues/11228
    $innoSetupPath = "C:\Program Files (x86)\Inno Setup 6\ISCC.exe"

    $definitions = @{
        "AppId"          = $appId
        "AppIconName"    = $appIconName
        "OutputDir"      = "$CargoTargetDir\release"
        "AppSetupName"   = $appSetupName
        "AppName"        = $appName
        "AppDisplayName" = $appDisplayName
        "RegValueName"   = $regValueName
        "AppMutex"       = $appMutex
        "AppExeName"     = $appExeName
        "ResourcesDir"   = "$innoDir"
        "ShellNameShort" = $appShellNameShort
        "AppUserId"      = $appUserId
        "UrlScheme"      = $env:ZED_PRODUCT_URL_SCHEME
        "Version"        = "$env:RELEASE_VERSION"
        "SourceDir"      = "$env:ZED_WORKSPACE"
    }
    if ($Comfy) {
        $definitions["Comfy"] = "1"
    }

    $defs = @()
    foreach ($key in $definitions.Keys) {
        $defs += "/d$key=`"$($definitions[$key])`""
    }

    $innoArgs = @($issFilePath) + $defs
    if($canCodeSign) {
        # Checked by zed.iss to decide whether to sign the installer.
        $env:ZED_SIGN_BUNDLE = "1"
        $signTool = "powershell.exe -ExecutionPolicy Bypass -File $innoDir\sign.ps1 `$f"
        $innoArgs += "/sDefaultsign=`"$signTool`""
    }

    # Execute Inno Setup
    Write-Host "🚀 Running Inno Setup: $innoSetupPath $innoArgs"
    $process = Start-Process -FilePath $innoSetupPath -ArgumentList $innoArgs -NoNewWindow -Wait -PassThru

    if ($process.ExitCode -eq 0) {
        $expectedSetupPath = "$CargoTargetDir\release\$appSetupName.exe"
        if (-not (Test-Path $expectedSetupPath -PathType Leaf)) {
            throw "Inno Setup did not produce the expected product artifact: $expectedSetupPath"
        }
        Write-Host "✅ Inno Setup successfully compiled the installer"
        Write-Output "SETUP_PATH=$CargoTargetDir/release/$appSetupName.exe" >> $env:GITHUB_ENV
        $script:buildSuccess = $true
    }
    else {
        Write-Host "❌ Inno Setup failed: $($process.ExitCode)"
        $script:buildSuccess = $false
    }
}

ParseZedWorkspace
$innoDir = "$env:ZED_WORKSPACE\inno\$Architecture"
$debugArchive = "$CargoOutDir\$env:ZED_PRODUCT_ID-$env:RELEASE_VERSION-$env:ZED_RELEASE_CHANNEL.dbg.zip"

CheckEnvironmentVariables
PrepareForBundle
GenerateLicenses
BuildProductBinaries
BuildRemoteServer
SignZedAndItsFriends
ZipProductDebugSymbols
DownloadAMDGpuServices
DownloadConpty
CollectFiles
BuildInstaller

if ($buildSuccess) {
    Write-Output "Build successful"
    if ($Install) {
        Write-Output "Installing $env:ZED_PRODUCT_DISPLAY_NAME..."
        Start-Process -FilePath "$CargoTargetDir/release/$env:ZED_PRODUCT_ARTIFACT_NAME"
    }
    exit 0
}
else {
    Write-Output "Build failed"
    exit 1
}
