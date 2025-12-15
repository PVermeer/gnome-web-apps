#!/bin/bash

set -e

is_release="false"
if [ "$1" == "release" ]; then
    is_release="true"
fi

if [ "$is_release" == "true" ]; then
    echo -e "\n==== Building Flatpak Release ====\n"

    cd flatpak-builder-tools/cargo
    poetry install
    eval "$(poetry env activate)"
    python3 flatpak-cargo-generator.py ../../Cargo.lock -o ../../flatpak/cargo-sources.json
    cd -

    flatpak-builder \
        --install-deps-from=flathub \
        --repo=target/flatpak-release/repo \
        --state-dir=target/flatpak-release/.flatpak-builder \
        --force-clean \
        --install \
        --user \
        --disable-rofiles-fuse \
        target/flatpak-release/build \
        flatpak/org.pvermeer.WebAppHub.yml

    echo -e "\n==== Building Bundle ====\n"

    flatpak build-bundle \
        target/flatpak-release/repo \
        target/flatpak-release/web-app-hub.flatpak \
        org.pvermeer.WebAppHub
else
    echo -e "\n==== Building Flatpak Devel ====\n"

    echo -e "\n==== Updating cargo vendors ====\n"
    cargo vendor target/flatpak-devel/vendor/ --locked

    flatpak-builder \
        --install-deps-from=flathub \
        --repo=target/flatpak-devel/repo \
        --state-dir=target/flatpak-devel/.flatpak-builder \
        --force-clean \
        --install \
        --user \
        --disable-rofiles-fuse \
        target/flatpak-devel/build \
        flatpak/org.pvermeer.WebAppHub.Devel.yml

    flatpak build-bundle \
        target/flatpak-devel/repo \
        target/flatpak-devel/web-app-hub.flatpak \
        org.pvermeer.WebAppHub
fi

echo -e "\n==== Done ====\n"
