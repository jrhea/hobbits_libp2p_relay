REM Run in Visual Studio x64 Native Tools Command Prompt
REM Needs vcpkg (install in sibling folder, next to mothra, for references to work)
REM Needs OpenSSL (use: vcpkg install openssl:x64-windows)
REM Needs Rust (also has a dependency on Visual Studio)

set EXT=dll
set ROOT_DIR=%~dp0..
set OUT_DIR=%ROOT_DIR%\bin
set VCPKG_DIR=%ROOT_DIR%\..\vcpkg
set OPENSSL_DIR=%VCPKG_DIR%\packages\openssl-windows_x64-windows

if not exist "%VCPKG_DIR%" goto :vcpkg_missing
if not exist "%OPENSSL_DIR%" goto :openssl_missing

@echo ## [Core] Cleaning "%OUT_DIR%\release"
rmdir /S /Q "%OUT_DIR%\release"

@echo ## Building FFI library to "%OUT_DIR%"
cargo build --release --target-dir="%OUT_DIR%"
if errorlevel 0 goto :copy
goto :end

:copy
@echo ## Copying dynamic libraries to "%OUT_DIR%"
robocopy "%OUT_DIR%\release" "%OUT_DIR%" "mothra.%EXT%"
robocopy "%OPENSSL_DIR%\bin" "%OUT_DIR%" "libcrypto*.*" "libssl*.*"
goto :end

:vcpkg_missing
@echo ## vcpkg is missing from "%VCPKG_DIR%"
@echo Clone from https://github.com/microsoft/vcpkg to sibling folder of Mothra and run bootstrap

:openssl_missing
@echo ## openssl is missing from "%OPENSSL_DIR%"
@echo Install with 'vcpkg install openssl:x64-windows'

:end