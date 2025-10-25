FROM fedora:latest

RUN sudo dnf update -y
RUN sudo dnf install -y git dnf-plugins-core
RUN sudo dnf install -y https://mirrors.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm https://mirrors.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
RUN sudo dnf config-manager setopt fedora-cisco-openh264.enabled=1
RUN sudo dnf swap -y ffmpeg-free ffmpeg --allowerasing
RUN sudo dnf swap -y mesa-va-drivers mesa-va-drivers-freeworld
RUN sudo dnf install -y rpmfusion-free-release-tainted
RUN sudo dnf install -y libdvdcss
RUN sudo dnf install -y @gnome-desktop rustup libadwaita-devel pipewire-utils

# Create the user
ARG USERNAME=vscode
ARG USER_UID=1000
ARG USER_GID=$USER_UID
RUN groupadd --gid $USER_GID $USERNAME \
    && useradd --uid $USER_UID --gid $USER_GID -m $USERNAME \
    && echo $USERNAME ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/$USERNAME \
    && chmod 0440 /etc/sudoers.d/$USERNAME
USER $USERNAME

RUN rustup-init -y
RUN mkdir -p /tmp/user/1000/gvfsd
