#!/bin/bash

set -e

is_release="false"
if [ "$1" == "release" ]; then
    is_release="true"
fi

if [ "$is_release" == "true" ]; then
    target_dir="target/flatpak-release"

    echo -e "\n==== Generating cargo sources ====\n"

    cd external/flatpak-builder-tools/cargo
    poetry install
    eval "$(poetry env activate)"
    python3 flatpak-cargo-generator.py ../../Cargo.lock -o ../../flatpak/cargo-sources.json
    cd -

    echo -e "\n==== Building Flatpak Release ====\n"

    flatpak-builder \
        --install-deps-from=flathub \
        --repo=$target_dir/repo \
        --state-dir=$target_dir/.flatpak-builder \
        --force-clean \
        --install \
        --user \
        --disable-rofiles-fuse \
        --disable-cache \
        --mirror-screenshots-url=https://dl.flathub.org/media/ \
        $target_dir/build \
        flatpak/org.pvermeer.WebAppHub.yml

    echo -e "\n==== Building Bundle ====\n"

    flatpak build-bundle \
        $target_dir/repo \
        $target_dir/web-app-hub.flatpak \
        org.pvermeer.WebAppHub
else
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
fi

echo -e "\n==== Done ====\n"
