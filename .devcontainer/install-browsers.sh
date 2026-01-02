#!/bin/bash

set -e

if [ "$1" != "--user" ] && [ "$1" != "--system" ]; then
    echo Please provide --user or --system argument
    exit 1
fi

echo "==== Starting browser installs"

# shellcheck disable=SC2016
vivaldi_repo='
[vivaldi]
name=vivaldi
enabled=1
baseurl=https://repo.vivaldi.com/archive/rpm/$basearch
gpgcheck=1
gpgkey=https://repo.vivaldi.com/archive/linux_signing_key.pub
'
echo "$vivaldi_repo" | sudo tee /etc/yum.repos.d/vivaldi-fedora.repo

opera_repo='
[opera]
name=Opera packages
type=rpm-md
baseurl=https://rpm.opera.com/rpm
gpgcheck=1
gpgkey=https://rpm.opera.com/rpmrepo.key
enabled=1
'
echo "$opera_repo" | sudo tee /etc/yum.repos.d/opera.repo

sudo dnf install -y dnf-plugins-core fedora-workstation-repositories
sudo dnf config-manager setopt google-chrome.enabled=1

system_repos=(
    "https://brave-browser-rpm-release.s3.brave.com/brave-browser.repo"
)

for repo in "${system_repos[@]}"; do
    sudo dnf config-manager addrepo -y --from-repofile="$repo" || true
done

system_browsers=(
    "chromium"
    "firefox"
    "brave-browser"
    "google-chrome-stable"
    "vivaldi-stable"
    "opera-stable"
)
sudo dnf install -y "${system_browsers[@]}"

flatpak_browsers=(
    "com.brave.Browser"
    "org.chromium.Chromium"
    "io.github.ungoogled_software.ungoogled_chromium"
    "com.google.Chrome"
    "org.gnome.Epiphany"
    "com.vivaldi.Vivaldi"
    "com.opera.Opera"
    "org.mozilla.firefox"
    "one.ablaze.floorp"
    "app.zen_browser.zen"
)
flatpak "$1" install -y "${flatpak_browsers[@]}"

echo "==== Done"
