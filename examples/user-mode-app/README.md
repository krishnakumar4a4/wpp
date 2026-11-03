# User-Mode App

A simple Rust console application that emits WPP trace messages through ETW.
IFR is kernel-mode only, so this app does not enable the `kernel_mode` feature
of `wpp`.

## Build and run

```powershell
cargo +nightly build
.\target\debug\user-mode-app.exe
```

## Capture traces

Run from an elevated prompt in this directory:

```powershell
logman start wpp-um -p "{6f1c2b8a-3d4e-4f5a-9b6c-7d8e9f0a1b2c}" 0xffffffff 5 -o wpp-um.etl -ets
.\target\debug\user-mode-app.exe
logman stop wpp-um -ets
```

## View traces

Use the WDK tools. Adjust `$bin` to match your installed WDK version.

```powershell
$bin = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'

# Extract trace message formats (TMF) from the PDB.
& "$bin\tracepdb.exe" -f .\target\debug\user_mode_app.pdb -p .\tmf

# Decode the capture to text.
& "$bin\tracefmt.exe" wpp-um.etl -p .\tmf -o wpp-um.txt -nosummary
Get-Content wpp-um.txt
```

The PDB must come from the same build that produced the capture. After
changing trace messages, rebuild, capture again, and rerun `tracepdb`.

For a GUI, open the ETL in `traceview.exe` from the same folder and select the
PDB when prompted.
