REM Run in Visual Studio x64 Native Tools Command Prompt
REM Needs dotnet core installed

set ROOT_DIR=%~dp0..\..
set OUT_DIR=%ROOT_DIR%\bin

@echo ## Cleaning "%OUT_DIR%\dotnet"
rmdir /S /Q "%OUT_DIR%\dotnet"

@echo ## Compiling "%OBJ%"
dotnet build Mothra.csproj -o "%OUT_DIR%\dotnet"
