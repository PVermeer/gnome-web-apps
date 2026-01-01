#!/bin/bash

set -e

export packages="\
    flatpak-builder \
    gh \
    build-essential \
    libglib2.0-dev \
    libgtk-4-dev \
    libadwaita-1-dev"

if [ "$1" == "--only-export-packages" ]; then
    return
fi

echo -e "\n==== Installing deps ====\n"

sudo apt-get update -y

sudo apt-get install -y "$packages"

echo -e "\n==== Done ====\n"
