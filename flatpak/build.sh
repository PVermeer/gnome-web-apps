#!/bin/bash

set -e

target_dir="target/flatpak-devel"

echo -e "\n==== Building Flatpak Devel ====\n"

echo -e "\n==== Updating cargo vendors ====\n"
cargo vendor target/flatpak-devel/vendor/ --locked

flatpak-builder \
    --install-deps-from=flathub \
    --repo=$target_dir/repo \
    --state-dir=$target_dir/.flatpak-builder \
    --force-clean \
    --install \
    --user \
    --disable-rofiles-fuse \
    --mirror-screenshots-url=https://dl.flathub.org/media/ \
    $target_dir/build \
    flatpak/org.pvermeer.WebAppHub.Devel.yml

flatpak build-bundle \
    $target_dir/repo \
    $target_dir/web-app-hub.flatpak \
    org.pvermeer.WebAppHub

echo -e "\n==== Done ====\n"
