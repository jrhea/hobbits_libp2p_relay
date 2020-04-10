REM Run in Visual Studio x64 Native Tools Command Prompt

set ROOT_DIR=%~dp0
set OUT_DIR=%ROOT_DIR%\bin
set BIND_DIR=%ROOT_DIR%\bindings
set CBIND_DIR=%BIND_DIR%\c
set FFI_DIR=%ROOT_DIR%\ffi
set EXAMPLES_DIR=%ROOT_DIR%\examples
set CEXAMPLE_DIR=%EXAMPLES_DIR%\c
set DOTNETEXAMPLE_DIR=%EXAMPLES_DIR%\dotnet

@echo ## Cleaning "%OUT_DIR%"
if exist "%OUT_DIR%" ( del "%OUT_DIR%" /y )

if not exist "%OUT_DIR%" ( mkdir "%OUT_DIR%" )

@echo.
@echo ## Building FFI
cd "%FFI_DIR%"
call "%FFI_DIR%\Make.cmd"

@echo.
@echo ## Building C example
cd "%CEXAMPLE_DIR%"
call "%CEXAMPLE_DIR%\Make.cmd"

@echo.
@echo ## Building dotnet example
cd "%DOTNETEXAMPLE_DIR%"
call "%DOTNETEXAMPLE_DIR%\Make.cmd"
