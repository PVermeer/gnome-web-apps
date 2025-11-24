#!/bin/bash

set -e

cargo vendor target/flatpak/vendor/

echo -e "\n==== Building Flatpak ====\n"

flatpak-builder \
    --install-deps-from=flathub \
    --repo=target/flatpak/repo \
    --state-dir=target/flatpak/.flatpak-builder \
    --force-clean \
    --install \
    --user \
    target/flatpak/build flatpak/manifest.yml

echo -e "\n==== Building Bundle ====\n"

flatpak build-bundle \
    target/flatpak/repo \
    target/flatpak/gnome-web-apps.flatpak \
    org.pvermeer.GnomeWebApps

echo -e "\n==== Done ====\n"
