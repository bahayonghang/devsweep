@echo off
echo MOCK npm %*
if "%~1"=="test" if "%DEVSWEEP_PROBE_FAIL%"=="test" exit /b 23
if "%~2"=="%DEVSWEEP_PROBE_FAIL%" exit /b 23
exit /b 0
