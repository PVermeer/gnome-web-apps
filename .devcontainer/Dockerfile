FROM fedora:latest

RUN sudo dnf update -y
RUN sudo dnf install -y git dnf-plugins-core
RUN sudo dnf install -y https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
RUN sudo dnf config-manager setopt fedora-cisco-openh264.enabled=1
RUN sudo dnf swap -y ffmpeg-free ffmpeg --allowerasing
RUN sudo dnf swap -y mesa-va-drivers mesa-va-drivers-freeworld
RUN sudo dnf install -y rpmfusion-free-release-tainted
RUN sudo dnf install -y libdvdcss
RUN sudo dnf install -y @gnome-desktop
RUN sudo dnf install -y dbus-launch

# Dev
RUN sudo dnf install -y flatpak-builder gh rustup make git clang python3-devel pipx python3-pip gtk4-devel libdrm-devel vulkan-tools OpenCL-ICD-Loader-devel boost-devel cmake gcc gcc-c++ intel-mediasdk-devel libappindicator-gtk3-devel libcap-devel libcurl-devel libdrm-devel libevdev-devel libnotify-devel libva-devel libvdpau-devel libX11-devel libxcb-devel libXcursor-devel libXfixes-devel libXi-devel libXinerama-devel libXrandr-devel libXtst-devel mesa-libGL-devel miniupnpc-devel npm numactl-devel openssl-devel opus-devel pulseaudio-libs-devel rpm-build pipewire-devel pipewire-utils doxygen libadwaita-devel shfmt shellcheck

# Don't use this, only --user, but for GTK4 and loading images in memory (PixBuf with glycin)
# it needs a valid repo setup for it to work with flatpaks.
# Why glycin is using a setup like this ¯\_(ツ)_/¯, who knows...
RUN flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo

# Custom installation for flatpak
RUN mkdir -p /etc/flatpak/installations.d; \
    touch /etc/flatpak/installations.d/some_custom_installation.conf; \
    echo -e '\
    [Installation "some_custom_installation"]\n\
    Path=/some_custom_installation/flatpak/\n\
    DisplayName=Some custom installation\n\
    StorageType=harddisk\n\
    ' > /etc/flatpak/installations.d/some_custom_installation.conf;

RUN flatpak --installation="some_custom_installation" remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo

# Add DBus user session startup for all login shells
RUN echo 'if [ -z "$DBUS_SESSION_BUS_ADDRESS" ]; then \
    eval $(dbus-launch --sh-syntax); \
    fi; \
    sudo rm /run/dbus/pid || true && sudo dbus-daemon --system || true;' > /etc/profile.d/dbus.sh
RUN chmod +x /etc/profile.d/dbus.sh

# Create the user
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME
USER $USERNAME

ENV APPIMAGE_EXTRACT_AND_RUN="1"

RUN rustup-init -y
RUN mkdir -p /tmp/user/1000/gvfsd
RUN sudo mkdir -p /run/dbus
RUN flatpak --user remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
RUN pipx install poetry
