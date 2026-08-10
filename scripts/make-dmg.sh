#!/bin/bash
# Fallback DMG packaging (tauri's styled bundle_dmg.sh needs interactive Finder).
set -euo pipefail
cd "$(dirname "$0")/../target/release/bundle"
codesign --force --deep -s - macos/ParaTwin.app
rm -rf dmg-stage && mkdir dmg-stage
cp -R macos/ParaTwin.app dmg-stage/
ln -s /Applications dmg-stage/Applications
hdiutil create -volname "ParaTwin" -srcfolder dmg-stage -ov -format UDZO ParaTwin_1.3.1_x64.dmg
rm -rf dmg-stage
echo "-> $(pwd)/ParaTwin_1.3.1_x64.dmg"
