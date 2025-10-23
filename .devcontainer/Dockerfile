FROM rust:1.83-bookworm

# Install system dependencies for building all platforms
RUN apt-get update && apt-get install -y \
    # Build essentials
    build-essential \
    pkg-config \
    libssl-dev \
    git \
    curl \
    wget \
    unzip \
    # Wine for running WiX on Linux
    wine \
    wine64 \
    winetricks \
    xvfb \
    cabextract \
    # NSIS native Linux compiler
    nsis \
    # Linux package tools
    rpm \
    fakeroot \
    dpkg-dev \
    # AppImage dependencies
    file \
    desktop-file-utils \
    # General utilities
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create build user matching typical host UID/GID
ARG USER_UID=1000
ARG USER_GID=1000

RUN groupadd --gid $USER_GID builder \
    && useradd --uid $USER_UID --gid $USER_GID -m builder \
    && apt-get update \
    && apt-get install -y sudo \
    && echo builder ALL=\(root\) NOPASSWD:ALL > /etc/sudoers.d/builder \
    && chmod 0440 /etc/sudoers.d/builder \
    && rm -rf /var/lib/apt/lists/*

# Switch to build user
USER builder
ENV HOME=/home/builder
ENV WINEPREFIX=$HOME/.wine
ENV WINEARCH=win64
ENV PATH=$HOME/.cargo/bin:$PATH

# Install Rust nightly (matching rust-toolchain.toml)
RUN rustup default nightly-2024-10-20 \
    && rustup component add rustfmt clippy

# Initialize Wine and install .NET Framework 4.0 (required for WiX)
# This is the time-consuming step but necessary for WiX to work
RUN wine wineboot --init && \
    echo "Installing .NET Framework 4.0 for WiX..." && \
    xvfb-run sh -c 'winetricks --unattended dotnet40 && wineserver -w' && \
    echo "Wine setup complete"

# Create cache directory for WiX/NSIS downloads
RUN mkdir -p $HOME/.cache/cyrup

WORKDIR /workspace

# Verify installations
RUN rustc --version && \
    cargo --version && \
    wine --version && \
    makensis -VERSION
