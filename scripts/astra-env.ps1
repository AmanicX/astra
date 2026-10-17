#!/usr/bin/env pwsh
# Wraps a command with the user-space build environment (GNU Rust toolchain + LLVM-MinGW).
param(
  [Parameter(Mandatory = $true, ValueFromRemainingArguments = $true)]
  [string[]]$Command
)

$Toolchain = "$env:USERPROFILE\toolchains\llvm-mingw-20261006-ucrt-x86_64"
$env:PATH = "$Toolchain\bin;$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CC_x86_64_pc_windows_gnu = "$Toolchain\bin\x86_64-w64-mingw32-gcc.exe"
$env:CXX_x86_64_pc_windows_gnu = "$Toolchain\bin\x86_64-w64-mingw32-g++.exe"
$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = "$Toolchain\bin\x86_64-w64-mingw32-gcc.exe"

# Tauri copies WebView2Loader.dll next to the app binary (src-tauri\target\debug)
# but cargo launches unit-test harnesses from target\debug\deps. Put the output
# directories on PATH so the loader can resolve the DLL in test processes too.
foreach ($profile in @("debug", "release")) {
  $outDir = Join-Path $PSScriptRoot "..\src-tauri\target\$profile"
  if (Test-Path $outDir) { $env:PATH = "$outDir;$env:PATH" }
}

# llvm-mingw ships libunwind + compiler-rt instead of libgcc, but Rust's
# windows-gnu std still passes -lgcc/_eh. Provide libunwind under those names.
$ShimDir = Join-Path $PSScriptRoot "..\src-tauri\.linker"
$Unwind = "$Toolchain\x86_64-w64-mingw32\lib\libunwind.a"
if (Test-Path $Unwind) {
  New-Item -ItemType Directory -Path $ShimDir -Force | Out-Null
  foreach ($name in @("libgcc.a", "libgcc_eh.a")) {
    $dest = Join-Path $ShimDir $name
    if (-not (Test-Path $dest) -or (Get-Item $dest).Length -ne (Get-Item $Unwind).Length) {
      Copy-Item -LiteralPath $Unwind -Destination $dest -Force
    }
  }
}

$env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS = "-L `"$ShimDir`""

# Invoke the wrapped command. Slice carefully: a single-argument command
# (e.g. `astra-env.ps1 npm`) has no `[1..1]` tail to index into.
$exe = $Command[0]
if ($Command.Length -gt 1) {
  & $exe $Command[1..($Command.Length - 1)]
} else {
  & $exe
}
exit $LASTEXITCODE
