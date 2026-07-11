# 1BitShit CPU Windows installer
# Installs CLI, engine, Llama kernel and runtime directories without removing
# existing Cluaiz data. Legacy data is copied once into the new runtime.

param([string]$Version = 'latest')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Net.ServicePointManager]::SecurityProtocol =
    [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

function Write-Step([string]$Message) {
    Write-Host ("  [....] " + $Message) -ForegroundColor DarkGray
}

function Write-Done([string]$Message) {
    Write-Host ("  [DONE] " + $Message) -ForegroundColor Green
}

function Write-Fail([string]$Message) {
    Write-Host ("  [ERROR] " + $Message) -ForegroundColor Red
}

function Invoke-BitShitDownload(
    [string]$Url,
    [string]$Destination,
    [string]$Label
) {
    if ([string]::IsNullOrWhiteSpace($Url)) {
        throw "Download URL is missing for $Label"
    }

    $Directory = Split-Path $Destination
    if (-not (Test-Path $Directory)) {
        New-Item -ItemType Directory -Path $Directory -Force | Out-Null
    }

    $Partial = "$Destination.part"
    if (Test-Path $Partial) {
        Remove-Item $Partial -Force
    }

    Write-Step "Downloading $Label"
    Invoke-WebRequest -Uri $Url -OutFile $Partial -UseBasicParsing
    if (-not (Test-Path $Partial)) {
        throw "Artifact retrieval failed for $Label"
    }

    if (Test-Path $Destination) {
        Remove-Item $Destination -Force
    }
    Move-Item $Partial $Destination -Force
    Write-Done "$Label installed"
}

function Copy-LegacyDirectory([string]$Source, [string]$Destination) {
    if (-not (Test-Path $Source)) {
        return
    }
    if (-not (Test-Path $Destination)) {
        New-Item -ItemType Directory -Path $Destination -Force | Out-Null
    }
    Copy-Item (Join-Path $Source '*') $Destination -Recurse -Force -ErrorAction SilentlyContinue
}

Clear-Host
Write-Host ''
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host '                    1BitShit CPU Runtime' -ForegroundColor Cyan
Write-Host '          Llama, GGUF, BitNet, ONNX and Native FFI' -ForegroundColor Cyan
Write-Host '  ============================================================' -ForegroundColor Cyan
Write-Host ''

try {
    $HubPath = if ($env:BITSHIT_HOME) {
        $env:BITSHIT_HOME
    }
    elseif ($env:BITSHIT_ROOT) {
        $env:BITSHIT_ROOT
    }
    else {
        Join-Path $HOME '.1bitshit'
    }

    $LegacyPath = Join-Path $HOME '.cluaiz'
    $MigrationMarker = Join-Path $HubPath '.legacy-cluaiz-import-complete'
    $Repository = 'eyshoit-commits/1bitshit-cpu'
    $RegistryUrl = "https://raw.githubusercontent.com/$Repository/main/package.json"
    $Architecture = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
        'win-arm64'
    }
    else {
        'win-x64'
    }

    Write-Step "Creating runtime directories at $HubPath"
    $Folders = @(
        'bin',
        'apps/cli',
        'engine',
        'engine/drivers',
        'models',
        'config',
        'brain',
        'skills',
        'extensions',
        'plugins',
        'mcp',
        'kv_cache',
        'reports'
    )
    foreach ($Folder in $Folders) {
        New-Item -ItemType Directory -Path (Join-Path $HubPath $Folder) -Force | Out-Null
    }
    Write-Done 'Runtime directories ready'

    if ((Test-Path $LegacyPath) -and -not (Test-Path $MigrationMarker)) {
        Write-Step 'Importing existing legacy data without deleting the old installation'
        foreach ($Folder in @(
            'models', 'brain', 'skills', 'extensions', 'plugins', 'mcp', 'kv_cache', 'reports'
        )) {
            Copy-LegacyDirectory `
                (Join-Path $LegacyPath $Folder) `
                (Join-Path $HubPath $Folder)
        }
        Set-Content -Path $MigrationMarker -Value (
            'Imported by 1BitShit CPU. The previous runtime was not executed or deleted.'
        )
        Write-Done 'Legacy data imported'
    }

    Write-Step 'Loading the 1BitShit CPU release registry'
    $MasterRegistry = Invoke-RestMethod -Uri $RegistryUrl
    Write-Done 'Registry loaded'

    $CliManifestUrl = $MasterRegistry.components.cli.manifest_url
    $CliManifest = Invoke-RestMethod -Uri $CliManifestUrl
    $CliUrl = $CliManifest.cli.$Architecture
    if (-not $CliUrl) {
        throw "No CLI artifact for $Architecture"
    }

    $TargetCli = Join-Path $HubPath 'apps/cli/bitshit.exe'
    Invoke-BitShitDownload $CliUrl $TargetCli "1BitShit CPU CLI ($Architecture)"

    $BinDirectory = Join-Path $HubPath 'bin'
    $BinLink = Join-Path $BinDirectory 'bitshit.exe'
    if (Test-Path $BinLink) {
        Remove-Item $BinLink -Force
    }

    Write-Step 'Creating global bitshit command'
    $LinkArguments = '/c mklink /H "' + $BinLink + '" "' + $TargetCli + '" >nul 2>&1'
    Start-Process -FilePath 'cmd.exe' -ArgumentList $LinkArguments -NoNewWindow -Wait
    if (-not (Test-Path $BinLink)) {
        Copy-Item $TargetCli $BinLink -Force
    }
    Write-Done 'Global command ready'

    $EngineManifestUrl = $MasterRegistry.components.engine.manifest_url
    if ($EngineManifestUrl) {
        $EngineManifest = Invoke-RestMethod -Uri $EngineManifestUrl
        $EngineUrl = $EngineManifest.engines.$Architecture
        if ($EngineUrl) {
            Invoke-BitShitDownload `
                $EngineUrl `
                (Join-Path $HubPath 'engine/bitshit-engine.dll') `
                "1BitShit CPU engine ($Architecture)"
        }
    }

    $KernelManifestUrl = $MasterRegistry.components.kernel.manifest_url
    if ($KernelManifestUrl) {
        $KernelManifest = Invoke-RestMethod -Uri $KernelManifestUrl
        $TargetPlatform = if ($Architecture -eq 'win-arm64') {
            'win-arm64'
        }
        elseif ($env:PROCESSOR_IDENTIFIER -like '*AVX512*') {
            'win-x64-avx512'
        }
        else {
            'win-x64-avx2'
        }

        $KernelUrl = $KernelManifest.kernels.$TargetPlatform
        if ($KernelUrl) {
            Invoke-BitShitDownload `
                $KernelUrl `
                (Join-Path $HubPath 'engine/bitshit-llama.dll') `
                "1BitShit Llama kernel ($TargetPlatform)"
            Set-Content `
                -Path (Join-Path $HubPath 'engine/bitshit-llama.ready') `
                -Value $KernelManifest.version
        }
    }

    [System.Environment]::SetEnvironmentVariable('BITSHIT_HOME', $HubPath, 'User')
    [System.Environment]::SetEnvironmentVariable('BITSHIT_ROOT', $HubPath, 'User')

    $UserPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
    if ($UserPath -notlike ('*' + $BinDirectory + '*')) {
        $UpdatedPath = if ([string]::IsNullOrWhiteSpace($UserPath)) {
            $BinDirectory
        }
        else {
            $UserPath + ';' + $BinDirectory
        }
        [System.Environment]::SetEnvironmentVariable('Path', $UpdatedPath, 'User')
    }

    Write-Host ''
    $BrainChoice = Read-Host 'Enable the native FFI memory brain? Type y or n'
    $BrainEnabled = if ($BrainChoice -match '^[yY]') { '1' } else { '0' }
    [System.Environment]::SetEnvironmentVariable('BITSHIT_FFI_BRAIN', $BrainEnabled, 'User')
    # Compatibility for older memory plugins. It is not shown or used as product identity.
    [System.Environment]::SetEnvironmentVariable('cluaizdb_FFI', $BrainEnabled, 'User')

    Write-Step 'Calibrating hardware'
    & $BinLink --calibrate
    Write-Done 'Hardware calibration complete'

    Write-Host ''
    Write-Done '1BitShit CPU deployment completed'
    Write-Host "  Runtime: $HubPath"
    Write-Host '  Command: bitshit'
    Write-Host ''

    & $BinLink
}
catch {
    Write-Fail ('Deployment failed: ' + $_.Exception.Message)
    Write-Host '  The previous installation and user models were not deleted.' -ForegroundColor DarkGray
    exit 1
}
