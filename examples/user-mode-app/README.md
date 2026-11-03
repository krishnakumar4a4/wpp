# User-Mode App

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
.\target\debug\user-mode-app.exe
```

## Capture traces

Run from an elevated prompt in this directory. Enable each provider whose
traces you want:

```powershell
logman start wpp-um -p "{34746ee9-2982-4967-aac1-fece7a3dfc3c}" 0xffffffff 5 -o wpp-um.etl -ets
logman update trace wpp-um -p "{c5a04b92-0ed8-45ec-9732-aa27fdc95e45}" 0xffffffff 5 -ets
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

Generates the following trace statements
```
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]User-mode app started, pid=32236
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Flags: initialized=0x00000001(true), safe_mode=0x00000000(false), build=safe-rust
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Direct Signed literals: i8=�, i16=-1600, i32=-320000, i64=-64000000000, isize=-42
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Direct Unsigned literals: u8=�, u16=60000, u32=4000000000, u64=18000000000000000000, usize=40960
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Signed literal variables: v1=�, v2=-1600, v3=-320000, v4=-64000000000, v5=-42
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Unsigned literal variables: u1=�, u2=60000, u3=4000000000, u4=18000000000000000000, u5=40960
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Custom enums: byte=0x00000002(Ready), short=0x0000000a(Warning), long=0x00000002(Recovering), implicit=0x00000002(Ready), mixed=0x0000000b(Failed)
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Processed item=1, running total=1
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Processed item=2, running total=3
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Processed item=3, running total=6
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Processed item=4, running total=10
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Processed item=5, running total=15
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Done: name=wpp-user-mode, total=15, ok=0x00000001(true)
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [SecondProvider]Generic values: first=123, second=another-generic-value
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [SecondProvider]Generic values: first=2147483647, second=32767
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [SecondProvider]Generic values: first=4294967295, second=65535
[8]7DEC.2C5C::11/03/2026-22:06:48.097 [UserModeApp]Default provider message without keyword
```
