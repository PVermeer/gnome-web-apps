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

RUN sudo dnf install -y flatpak-builder rustup make git clang python3-pip gtk4-devel libdrm-devel vulkan-tools OpenCL-ICD-Loader-devel boost-devel cmake gcc gcc-c++ intel-mediasdk-devel libappindicator-gtk3-devel libcap-devel libcurl-devel libdrm-devel libevdev-devel libnotify-devel libva-devel libvdpau-devel libX11-devel libxcb-devel libXcursor-devel libXfixes-devel libXi-devel libXinerama-devel libXrandr-devel libXtst-devel mesa-libGL-devel miniupnpc-devel npm numactl-devel openssl-devel opus-devel pulseaudio-libs-devel rpm-build pipewire-devel pipewire-utils doxygen libadwaita-devel

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