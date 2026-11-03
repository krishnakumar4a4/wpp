# Test App

A simple Rust console application that emits WPP trace messages through ETW.
IFR is kernel-mode only, so this app does not enable the `kernel_mode` feature
of `wpp`.

It traces literals, variables, C strings, booleans, custom enums (via
`#[derive(wpp::WppField)]`), generic values, and a message without a keyword.

| Provider | Control GUID | Keywords |
|---|---|---|
| `UserModeApp` | `34746ee9-2982-4967-aac1-fece7a3dfc3c` | `GENERAL`, `LITERALS`, `VARIABLES`, `WDF_TYPES`, `ENUMS`, `WORK` |
| `SecondProvider` | `c5a04b92-0ed8-45ec-9732-aa27fdc95e45` | `GENERICS`, `DEFAULT` |

Control GUIDs in `wpp_control_guids!` must be quoted string literals.

## Build and run

```powershell
cargo +nightly build
.\target\debug\test-app.exe
```

## Capture traces

Run from an elevated prompt in this directory. Enable each provider whose
traces you want:

```powershell
logman start wpp-um -p "{34746ee9-2982-4967-aac1-fece7a3dfc3c}" 0xffffffff 5 -o wpp-um.etl -ets
logman update trace wpp-um -p "{c5a04b92-0ed8-45ec-9732-aa27fdc95e45}" 0xffffffff 5 -ets
.\target\debug\test-app.exe
logman stop wpp-um -ets
```

## View traces

Use the WDK tools. Adjust `$bin` to match your installed WDK version.

```powershell
$bin = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64'

# Extract trace message formats (TMF) from the PDB.
& "$bin\tracepdb.exe" -f .\target\debug\test_app.pdb -p .\tmf

# Decode the capture to text.
& "$bin\tracefmt.exe" wpp-um.etl -p .\tmf -o wpp-um.txt -nosummary
Get-Content wpp-um.txt
```

The PDB must come from the same build that produced the capture. After
changing trace messages, rebuild, capture again, and rerun `tracepdb`.

For a GUI, open the ETL in `traceview.exe` from the same folder and select the
PDB when prompted.
