REM Run in Visual Studio x64 Native Tools Command Prompt

set ROOT_DIR=%~dp0..\..
set OUT_DIR=%ROOT_DIR%\bin
set BIND_DIR=%ROOT_DIR%\bindings
set CBIND_DIR=%BIND_DIR%\c

set CFLAGS=/W4 /O2 /EHsc /MD
set LFLAGS=
set OBJ=%OUT_DIR%\mothra-c.obj
set TARGET=%OUT_DIR%\mothra-c.lib

if not exist "%OUT_DIR%" ( mkdir "%OUT_DIR%" )

@echo ## Cleaning "%TARGET%"
if exist "%TARGET%" ( del "%TARGET%" )

@echo ## Compiling "%OBJ%"
cl /c /LD %CFLAGS% "%CBIND_DIR%/mothra-c.c" /Fo"%OBJ%"

@echo ## Creating static c library "%TARGET%"
lib %LFLAGS% "%OBJ%" /out:"%TARGET%"
