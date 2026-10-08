@echo off
rem Runs ReRAC with Goku in place of Ratchet (see GOKU.md).
rem Starts directly in Veldin, without menus or cutscenes: remove the three RC_ lines below for the normal game.
cd /d "%~dp0"
set RC_HERO_GLB=goku.glb
set RC_LEVEL=1
set RC_FRONTEND=0
set RC_SCENE=0
cargo dev -j 4
