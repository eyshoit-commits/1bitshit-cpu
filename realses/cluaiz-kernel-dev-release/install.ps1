# CLUAIZ CORE INFRASTRUCTURE - VERSION 0.1.0
# Industrial Standard Deployment Script (CURL ENHANCED)

param ([string]$Version = 'latest')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'

# --- UI Matrix ---
$E = [char]27
$BOLD = "$E[1m"; $CYAN = "$E[36m"; $GRAY = "$E[90m"; $GREEN = "$E[32m"; $YELLOW = "$E[33m"; $RED = "$E[31m"; $NC = "$E[0m"

# Professional UI Helpers (Pure ASCII - Industrial)
function Write-Step ([string]$msg) {
    # Initial state: Grey dot with message (No dots at end)
    Write-Host ("  " + $GRAY + "* " + $msg + $NC) -NoNewline
}

function Complete-Step ([string]$msg) {
    # Replaces the whole line with a Green [DONE] status + Message
    $clear = "`r" + (" " * 100) + "`r"
    Write-Host -NoNewline $clear
    Write-Host ("  " + $GREEN + "[DONE] " + $NC + $msg)
}

function Write-Success ([string]$msg) { 
    Write-Host ("`n  " + $GREEN + "[DONE] " + $msg + $NC)
}

function Write-Fail ([string]$msg) { 
    Write-Host ("`n  " + $RED + "[ERROR] " + $msg + $NC) -ForegroundColor Red
}

# --- High-Performance Download Engine (With Sequential Spinner) ---
function Invoke-CluaizDownload ([string]$url, [string]$path, [string]$label) {
    if (-not $url) { throw 'Download URL is null for ' + $label }
    $dir = Split-Path $path
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    
    # 🌀 Spinner Animation Logic
    $spinner = @('|', '/', '-', '\')
    $i = 0
    
    # Start download in background using WebClient for async UI
    $webClient = New-Object System.Net.WebClient
    $webClient.DownloadFileAsync($url, $path)
    
    # We strip any prefix for clean display
    $cleanLabel = $label -replace '\[MOUNTING\] ', ''
    
    while ($webClient.IsBusy) {
        $char = $spinner[$i % 4]
        # Overwrite the line with current spinner + DOWNLOADING status (No dots)
        $status = "`r  " + $CYAN + "[" + $char + "]" + $NC + " [DOWNLOADING] " + $cleanLabel
        Write-Host -NoNewline $status
        $i++
        Start-Sleep -Milliseconds 150
    }
    
    # Check if download actually finished successfully
    if (-not (Test-Path $path)) { throw "Artifact retrieval failed for $cleanLabel" }
    
    # Clear the spinner line completely before showing MOUNTED
    $clear = "`r" + (" " * 100) + "`r"
    Write-Host -NoNewline $clear
    Write-Host ("  " + $GREEN + "[MOUNTED] " + $NC + $cleanLabel)
}

# --- UTF-8 Safe ---
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

# --- Security ---
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12 -bor [Net.SecurityProtocolType]::Tls13

Clear-Host

# --- Unicode Safe Chars (As User Defined) ---
$C1 = [char]0x2591  # ░
$C2 = [char]0x2580  # ▀
$C3 = [char]0x2584  # ▄
$C4 = [char]0x2588  # █

# --- Logo ---
$Logo1 = "  $C1$C2$C3$C1$C1$C1$C1$C1$C1$C1$C1$C4$C2$C2$C1$C4$C1$C1$C1$C4$C1$C4$C1$C4$C2$C4$C1$C2$C4$C2$C1$C2$C2$C4"
$Logo2 = "  $C1$C1$C3$C2$C1$C1$C1$C1$C1$C1$C1$C4$C1$C1$C1$C4$C1$C1$C1$C4$C1$C4$C1$C4$C2$C4$C1$C1$C4$C1$C1$C3$C2$C1"
$Logo3 = "  $C1$C2$C1$C1$C1$C2$C2$C2$C1$C1$C1$C2$C2$C2$C1$C2$C2$C2$C1$C2$C2$C2$C1$C2$C1$C2$C1$C2$C2$C2$C1$C2$C2$C2"

# --- Print Logo ---
Write-Host ""
Write-Host $Logo1 -ForegroundColor Cyan
Write-Host $Logo2 -ForegroundColor Cyan
Write-Host $Logo3 -ForegroundColor Cyan

# --- Header ---
Write-Host ""
Write-Host "  >_ Installing Cluaiz..." -ForegroundColor Gray
Write-Host ""

try {
    $HubPath = if ($env:CLUAIZ_ROOT) { $env:CLUAIZ_ROOT } else { Join-Path $HOME '.cluaiz' }
    $Repo = 'cluaiz/cluaiz'

    # 1. Provisioning
    $step1 = '[PROVISIONING] Silicon Environment Setup'
    Write-Step $step1
    $Folders = @('bin', 'apps/cli', 'engine', 'interface-engines', 'interface-engines/kernels', 'interface-engines/drivers')
    foreach ($f in $Folders) {
        $p = Join-Path $HubPath $f
        if (-not (Test-Path $p)) { New-Item -ItemType Directory -Path $p -Force | Out-Null }
    }
    Complete-Step $step1

    # 2. Registry Discovery
    $step2 = '[AUDITING] Neural Registry Sync'
    Write-Step $step2
    $Releases = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases"
    $Arch = if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'win-arm64' } else { 'win-x64' }
    Complete-Step $step2

    # --- CLI Deployment ---
    $CliRel = $Releases | Where-Object { $_.tag_name -like 'cli-v*' -or $_.tag_name -eq 'cli-dev-release' } | Select-Object -First 1
    if (-not $CliRel) { throw 'No CLI release found in registry.' }
    
    # Direct Asset Lookup (Bypasses missing cli-manifest.json)
    $CliAsset = $CliRel.assets | Where-Object { $_.name -like "*$Arch*" } | Select-Object -First 1
    if (-not $CliAsset) { throw "No CLI asset matching $Arch found in release." }
    $CliUrl = $CliAsset.browser_download_url
    
    $TargetCli = Join-Path $HubPath 'apps/cli/cluaiz.exe'
    $CliLabel = "Cluaiz CLI ($Arch) $($CliRel.tag_name) - latest"
    Invoke-CluaizDownload -url $CliUrl -path $TargetCli -label $CliLabel
    
    # 🚀 Zero-Copy Linkage
    $BinPath = Join-Path $HubPath 'bin'
    $BinLink = Join-Path $BinPath 'cluaiz.exe'
    $step3 = 'Linking CLI Gateway'
    Write-Step $step3
    if (Test-Path $BinLink) { Remove-Item $BinLink -Force }
    $cmdArgs = '/c mklink /H "' + $BinLink + '" "' + $TargetCli + '" >nul 2>&1'
    Start-Process -FilePath 'cmd.exe' -ArgumentList $cmdArgs -NoNewWindow -Wait
    if (-not (Test-Path $BinLink)) { throw 'Hardlink creation failed.' }
    Complete-Step $step3

    # --- Engine Deployment ---
    $EngRel = $Releases | Where-Object { $_.tag_name -like 'engine-v*' -or $_.tag_name -eq 'engine-dev-release' } | Select-Object -First 1
    if (-not $EngRel) { throw 'No Engine release found in registry.' }
    
    # Direct Asset Lookup (Bypasses engine-manifest.json dependency)
    $EngAsset = $EngRel.assets | Where-Object { $_.name -like "*$Arch*" } | Select-Object -First 1
    if (-not $EngAsset) { throw "No Engine asset matching $Arch found in release." }
    $EUrl = $EngAsset.browser_download_url
    
    $EngLabel = "Cluaiz Engine ($Arch) $($EngRel.tag_name) - latest"
    Invoke-CluaizDownload -url $EUrl -path (Join-Path $HubPath 'engine/cluaiz-engine.dll') -label $EngLabel

    # --- Multi-Kernel Sync ---
    $KerRel = $Releases | Where-Object { $_.tag_name -like 'kernel-v*' -or $_.tag_name -eq 'kernel-dev-release' } | Select-Object -First 1
    if ($KerRel) {
        # Check CPU ISA to download AVX512 or AVX2 dynamically
        $AVX512_Enabled = $false
        if ($env:PROCESSOR_IDENTIFIER -like "*AVX512*") { $AVX512_Enabled = $true }
        
        $TargetPlatform = if ($AVX512_Enabled) { "win-x64-avx512" } else { "win-x64-avx2" }
        if ($Arch -eq 'win-arm64') { $TargetPlatform = 'win-arm64' } # Align ARM64
        
        # Direct Asset Lookup (Bypasses missing kernel-manifest.json entirely!)
        $KerAsset = $KerRel.assets | Where-Object { $_.name -like "*$TargetPlatform*" } | Select-Object -First 1
        if ($KerAsset) {
            $KUrl = $KerAsset.browser_download_url
            $KName = 'cluaiz-llama.dll'
            $KerLabel = "Cluaiz Llama Kernel ($TargetPlatform) $($KerRel.tag_name) - latest"
            Invoke-CluaizDownload -url $KUrl -path (Join-Path $HubPath "interface-engines/kernels/$KName") -label $KerLabel
        } else {
            Write-Host "  [Warning] No matching kernel asset found for $TargetPlatform in release. Continuing bootstrap..." -ForegroundColor Yellow
        }
    }

    # ── Environment Path Update ──────────────────────────────────────────
    [System.Environment]::SetEnvironmentVariable('CLUAIZ_ROOT', $HubPath, 'User')
    $OldPath = [System.Environment]::GetEnvironmentVariable('Path', 'User')
    if ($OldPath -notlike ('*' + $BinPath + '*')) {
        $NewPath = $OldPath + ';' + $BinPath
        [System.Environment]::SetEnvironmentVariable('Path', $NewPath, 'User')
    }

    Write-Host ("`n  " + $GREEN + "[DONE] Deployment successful." + $NC)
    
    # 🧬 Pre-Flight Calibration: Generate SiliconTruth before first boot
    Write-Host '>_ Synchronizing Hardware DNA...' -ForegroundColor Cyan
    & $BinLink --calibrate
    
    Write-Host '>_ Launching Cluaiz CLI...' -ForegroundColor Gray
    & $BinLink
}
catch {
    Write-Fail ('Deployment failed: ' + $_.Exception.Message)
    Write-Host "`n  [Troubleshoot] Check your connection." -ForegroundColor Gray
    Write-Host '  Press any key to exit...' -ForegroundColor Gray
    if ($Host.UI.RawUI) {
        $null = $Host.UI.RawUI.ReadKey('NoEcho,IncludeKeyDown')
    }
}