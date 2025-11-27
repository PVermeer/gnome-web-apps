#!/bin/bash

set -e

echo -e "\n==== Updating cargo vendors ====\n"

cargo vendor target/flatpak/vendor/ --locked

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
