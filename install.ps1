param(
    [ValidateSet('auto','cpu','cuda')]
    [string]$Backend = 'auto',
    [switch]$Yes,
    [switch]$NoMigrate,
    [switch]$NoLegacyAlias,
    [switch]$NoLaunch
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$InstallerVersion = '0.2.0'
$Repo = 'eyshoit-commits/1bitshit-cpu'
$RegistryUrl = if ($env:BITSHIT_REGISTRY_URL) { $env:BITSHIT_REGISTRY_URL } else { "https://raw.githubusercontent.com/$Repo/main/package.json" }
$BitShitHome = if ($env:BITSHIT_HOME) { $env:BITSHIT_HOME } else { Join-Path $HOME '.bitshit' }
$LegacyHome = if ($env:CLUAIZ_LEGACY_HOME) { $env:CLUAIZ_LEGACY_HOME } else { Join-Path $HOME '.cluaiz' }
$BinDir = Join-Path $BitShitHome 'bin'
$CliDir = Join-Path $BitShitHome 'apps\cli'
$EngineDir = Join-Path $BitShitHome 'engine'
$KernelDir = Join-Path $BitShitHome 'interface-engines\kernels'
$DriverDir = Join-Path $BitShitHome 'interface-engines\drivers'

function Step([string]$Message) { Write-Host "[bitshit] $Message" }
function Fail([string]$Message) { throw "[bitshit] $Message" }

function Get-RequiredProperty($Object, [string[]]$Path, [string]$Label) {
    $Value = $Object
    foreach ($Part in $Path) {
        if ($null -eq $Value) { Fail "Missing $Label" }
        $Property = $Value.PSObject.Properties[$Part]
        if ($null -eq $Property) { Fail "Missing $Label" }
        $Value = $Property.Value
    }
    if ($null -eq $Value -or [string]::IsNullOrWhiteSpace([string]$Value)) { Fail "Missing $Label" }
    return $Value
}

function Download-Artifact([string]$Url, [string]$Target, [string]$Label) {
    if ([string]::IsNullOrWhiteSpace($Url)) { Fail "Missing download URL for $Label" }
    $Directory = Split-Path -Parent $Target
    New-Item -ItemType Directory -Force -Path $Directory | Out-Null
    $Temp = "$Target.part"
    Remove-Item -Force $Temp -ErrorAction SilentlyContinue
    Step "Downloading $Label"
    Invoke-WebRequest -Uri $Url -OutFile $Temp -UseBasicParsing
    if (-not (Test-Path $Temp) -or (Get-Item $Temp).Length -eq 0) { Fail "Downloaded empty artifact for $Label" }
    Move-Item -Force $Temp $Target
}

function Copy-MissingTree([string]$Source, [string]$Target) {
    if (-not (Test-Path $Source -PathType Container)) { return }
    New-Item -ItemType Directory -Force -Path $Target | Out-Null
    Get-ChildItem -LiteralPath $Source -Recurse -Force | ForEach-Object {
        $Relative = $_.FullName.Substring($Source.Length).TrimStart('\','/')
        $Destination = Join-Path $Target $Relative
        if ($_.PSIsContainer) {
            New-Item -ItemType Directory -Force -Path $Destination | Out-Null
        }
        elseif (-not (Test-Path $Destination)) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Destination) | Out-Null
            Copy-Item -LiteralPath $_.FullName -Destination $Destination
        }
    }
}

if ($env:OS -ne 'Windows_NT') { Fail 'install.ps1 supports Windows only' }

if ($Backend -eq 'auto') {
    $Backend = if (Get-Command nvidia-smi.exe -ErrorAction SilentlyContinue) { 'cuda' } else { 'cpu' }
}
if ($Backend -eq 'cuda' -and -not (Get-Command nvidia-smi.exe -ErrorAction SilentlyContinue)) {
    Fail 'CUDA backend selected, but nvidia-smi.exe was not found'
}

$Platform = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'win-arm64' } else { 'win-x64' }
Step "Installing BitShit $InstallerVersion ($Platform, backend=$Backend)"
New-Item -ItemType Directory -Force -Path $BitShitHome,$BinDir,$CliDir,$EngineDir,$KernelDir,$DriverDir | Out-Null

if (-not $NoMigrate -and $LegacyHome -ne $BitShitHome -and (Test-Path $LegacyHome -PathType Container)) {
    Step "Importing missing legacy data from $LegacyHome"
    Copy-MissingTree -Source $LegacyHome -Target $BitShitHome
}

Step 'Loading package registry'
$Master = Invoke-RestMethod -Uri $RegistryUrl
$CliManifestUrl = Get-RequiredProperty $Master @('components','cli','manifest_url') 'components.cli.manifest_url'
$EngineManifestUrl = Get-RequiredProperty $Master @('components','engine','manifest_url') 'components.engine.manifest_url'
$KernelManifestUrl = Get-RequiredProperty $Master @('components','kernel','manifest_url') 'components.kernel.manifest_url'
$CliManifest = Invoke-RestMethod -Uri $CliManifestUrl
$EngineManifest = Invoke-RestMethod -Uri $EngineManifestUrl
$KernelManifest = Invoke-RestMethod -Uri $KernelManifestUrl

$CliUrl = Get-RequiredProperty $CliManifest @('cli',$Platform) "CLI asset for $Platform"
$EngineUrl = Get-RequiredProperty $EngineManifest @('engines',$Platform) "engine asset for $Platform"

$KernelPlatform = if ($Platform -eq 'win-arm64') {
    'win-arm64'
}
elseif ($env:PROCESSOR_IDENTIFIER -match 'AVX512') {
    'win-x64-avx512'
}
else {
    'win-x64-avx2'
}
$KernelUrl = Get-RequiredProperty $KernelManifest @('kernels',$KernelPlatform) "kernel asset for $KernelPlatform"

$CliTarget = Join-Path $CliDir 'bitshit.exe'
$BinTarget = Join-Path $BinDir 'bitshit.exe'
Download-Artifact $CliUrl $CliTarget "BitShit CLI ($Platform)"
Copy-Item -Force $CliTarget $BinTarget
if (-not $NoLegacyAlias) {
    Copy-Item -Force $CliTarget (Join-Path $BinDir 'cluaiz.exe')
}

Download-Artifact $EngineUrl (Join-Path $EngineDir 'bitshit-engine.dll') "BitShit engine ($Platform)"
Download-Artifact $KernelUrl (Join-Path $KernelDir 'bitshit-llama.dll') "BitShit kernel ($KernelPlatform)"

if ($Backend -eq 'cuda') {
    $DriverManifestUrl = Get-RequiredProperty $Master @('components','drivers','manifest_url') 'components.drivers.manifest_url'
    $DriverManifest = Invoke-RestMethod -Uri $DriverManifestUrl
    $DriverUrl = $null
    foreach ($Candidate in @("$Platform-cuda", $Platform)) {
        $Property = $DriverManifest.drivers.PSObject.Properties[$Candidate]
        if ($null -ne $Property -and -not [string]::IsNullOrWhiteSpace([string]$Property.Value)) {
            $DriverUrl = [string]$Property.Value
            break
        }
    }
    if (-not $DriverUrl) { Fail "Driver manifest has no CUDA asset for $Platform" }
    Download-Artifact $DriverUrl (Join-Path $DriverDir 'bitshit-cuda.dll') "BitShit CUDA driver ($Platform)"
}

$env:BITSHIT_HOME = $BitShitHome
$env:CLUAIZ_HOME = $BitShitHome
$env:BITSHIT_BACKEND = $Backend
$env:Path = "$BinDir;$env:Path"
[Environment]::SetEnvironmentVariable('BITSHIT_HOME', $BitShitHome, 'User')
[Environment]::SetEnvironmentVariable('CLUAIZ_HOME', $BitShitHome, 'User')
[Environment]::SetEnvironmentVariable('BITSHIT_BACKEND', $Backend, 'User')
$UserPath = [Environment]::GetEnvironmentVariable('Path','User')
if ([string]::IsNullOrWhiteSpace($UserPath)) { $UserPath = '' }
if (($UserPath -split ';') -notcontains $BinDir) {
    [Environment]::SetEnvironmentVariable('Path', (($UserPath.TrimEnd(';') + ';' + $BinDir).TrimStart(';')), 'User')
}

$InstallState = [ordered]@{
    product = 'bitshit'
    installer_version = $InstallerVersion
    platform = $Platform
    backend = $Backend
    home = $BitShitHome
    legacy_home = $LegacyHome
    binary = $BinTarget
}
$InstallState | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $BitShitHome 'install.json')

& $BinTarget --version | Out-Null
if ($LASTEXITCODE -ne 0) { Fail 'Installed BitShit binary failed its smoke test' }
Step "Installed $BinTarget"
Step "Runtime home: $BitShitHome"

if (-not $NoLaunch) {
    & $BinTarget --calibrate
    if ($LASTEXITCODE -ne 0) { Fail 'Calibration failed' }
    if (-not $Yes) { & $BinTarget }
}
