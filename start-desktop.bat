@echo off
set "GIT_USR_BIN=C:\Program Files\Git\usr\bin"
if exist "%GIT_USR_BIN%" (
    set "PATH=%GIT_USR_BIN%;%PATH%"
)
cd yntra-ui
dx serve --platform desktop
