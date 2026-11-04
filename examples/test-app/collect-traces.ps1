#Requires -Version 5.1

[CmdletBinding()]
param(
    [string]$BuildDirectory,
    [string]$WdkBinPath
)

$ErrorActionPreference = 'Stop'
$PSNativeCommandUseErrorActionPreference = $false

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Run this script from an administrator terminal (Run as administrator).'
}

function Find-TraceTool {
    param([string]$Name)

    if ($WdkBinPath) {
        $path = Join-Path $WdkBinPath $Name
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Cannot find $Name in '$WdkBinPath'. Specify the WDK tools directory with -WdkBinPath."
        }
        return (Resolve-Path -LiteralPath $path).Path
    }

    $command = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if ($command) {
        return $command.Source
    }

    $kitsBin = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (Test-Path -LiteralPath $kitsBin -PathType Container) {
        $versions = Get-ChildItem -LiteralPath $kitsBin -Directory |
            Where-Object { $_.Name -match '^\d+\.\d+\.\d+\.\d+$' } |
            Sort-Object { [version]$_.Name } -Descending
        $directories = @($versions | ForEach-Object { $_.FullName }) + @($kitsBin)
        foreach ($directory in $directories) {
            foreach ($architecture in @('x64', 'x86')) {
                $path = Join-Path $directory "$architecture\$Name"
                if (Test-Path -LiteralPath $path -PathType Leaf) {
                    return $path
                }
            }
        }
    }

    throw "Cannot find $Name. Install the WDK and specify its tools directory with -WdkBinPath."
}

function Invoke-TraceCommand {
    param(
        [string]$FilePath,
        [string[]]$Arguments
    )

    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "'$FilePath' failed with exit code $LASTEXITCODE."
    }
}

if (-not $BuildDirectory) {
    $BuildDirectory = Join-Path $PSScriptRoot 'target\debug'
}
$BuildDirectory = (Resolve-Path -LiteralPath $BuildDirectory).Path
$executable = Join-Path $BuildDirectory 'test-app.exe'
$pdb = Join-Path $BuildDirectory 'test_app.pdb'
foreach ($path in @($executable, $pdb)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Cannot find '$path'. Build the test app with cargo +nightly build first."
    }
}

$logman = (Get-Command logman.exe -CommandType Application -ErrorAction Stop |
    Select-Object -First 1).Source
$tracepdb = Find-TraceTool 'tracepdb.exe'
$tracefmt = Find-TraceTool 'tracefmt.exe'
$artifacts = Join-Path $PSScriptRoot 'target\trace-artifacts'
$tmf = Join-Path $artifacts 'tmf'
$etl = Join-Path $artifacts 'wpp-um.etl'
$text = Join-Path $artifacts 'wpp-um.txt'
$sessionName = "wpp-test-app-$([guid]::NewGuid().ToString('N'))"

if (Test-Path -LiteralPath $artifacts) {
    $item = Get-Item -LiteralPath $artifacts -Force
    if (-not $item.PSIsContainer -or ($item.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
        throw "Refusing to clear '$artifacts': expected a regular directory, not a file or link."
    }
    Remove-Item -LiteralPath $artifacts -Recurse -Force
}
New-Item -ItemType Directory -Path $tmf -Force | Out-Null

Push-Location -LiteralPath $artifacts
try {
    Write-Host "Extracting trace formats from $pdb"
    Invoke-TraceCommand $tracepdb @('-f', $pdb, '-p', $tmf)
    if (-not (Get-ChildItem -LiteralPath $tmf -Filter '*.tmf' -File)) {
        throw "tracepdb produced no TMF files. Rebuild the app with WPP annotations in '$pdb'."
    }

    $sessionStarted = $false
    try {
        Write-Host "Starting trace session $sessionName"
        Invoke-TraceCommand $logman @(
            'start', $sessionName,
            '-p', '{34746ee9-2982-4967-aac1-fece7a3dfc3c}', '0xffffffff', '5',
            '-o', $etl, '-ets'
        )
        $sessionStarted = $true
        Invoke-TraceCommand $logman @(
            'update', 'trace', $sessionName,
            '-p', '{c5a04b92-0ed8-45ec-9732-aa27fdc95e45}', '0xffffffff', '5',
            '-ets'
        )

        Write-Host "Running $executable"
        Invoke-TraceCommand $executable @()
    }
    finally {
        if ($sessionStarted) {
            Write-Host "Stopping trace session $sessionName"
            Invoke-TraceCommand $logman @('stop', $sessionName, '-ets')
        }
    }

    if (-not (Test-Path -LiteralPath $etl -PathType Leaf)) {
        throw "logman did not create the expected capture '$etl'."
    }

    Write-Host 'Decoded traces:'
    Invoke-TraceCommand $tracefmt @($etl, '-p', $tmf, '-o', $text, '-display', '-nosummary')
    if (-not (Test-Path -LiteralPath $text -PathType Leaf)) {
        throw "tracefmt did not create the expected output '$text'."
    }
    Write-Host "Trace artifacts saved in $artifacts"

    Get-Content -LiteralPath $text
}
finally {
    Pop-Location
}
