# Test environment with all supported Linux editors
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive
ENV DISPLAY=:99

# Install base dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libglib2.0-dev \
    libgtk-3-dev \
    libwebkitgtk-6.0-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    curl \
    wget \
    gnupg2 \
    software-properties-common \
    apt-transport-https \
    ca-certificates \
    git \
    xvfb \
    x11-utils \
    xdotool \
    wmctrl \
    procps \
    lsof \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Configure git for tests
RUN git config --global user.email "test@example.com" && \
    git config --global user.name "Test User"

# Install VSCode
RUN wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > packages.microsoft.gpg && \
    install -D -o root -g root -m 644 packages.microsoft.gpg /etc/apt/keyrings/packages.microsoft.gpg && \
    sh -c 'echo "deb [arch=amd64,arm64,armhf signed-by=/etc/apt/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" > /etc/apt/sources.list.d/vscode.list' && \
    apt-get update && \
    apt-get install -y code && \
    rm -f packages.microsoft.gpg && \
    rm -rf /var/lib/apt/lists/*

# Install VSCodium
RUN wget -qO - https://gitlab.com/paulcarroty/vscodium-deb-rpm-repo/raw/master/pub.gpg | gpg --dearmor | dd of=/usr/share/keyrings/vscodium-archive-keyring.gpg && \
    echo 'deb [signed-by=/usr/share/keyrings/vscodium-archive-keyring.gpg] https://download.vscodium.com/debs vscodium main' | tee /etc/apt/sources.list.d/vscodium.list && \
    apt-get update && \
    apt-get install -y codium && \
    rm -rf /var/lib/apt/lists/*

# Install Sublime Text
RUN wget -qO - https://download.sublimetext.com/sublimehq-pub.gpg | gpg --dearmor | tee /etc/apt/trusted.gpg.d/sublimehq-archive.gpg > /dev/null && \
    echo "deb https://download.sublimetext.com/ apt/stable/" | tee /etc/apt/sources.list.d/sublime-text.list && \
    apt-get update && \
    apt-get install -y sublime-text && \
    rm -rf /var/lib/apt/lists/*

# Install terminal-based editors
RUN apt-get update && apt-get install -y \
    vim \
    neovim \
    emacs-nox \
    nano \
    kakoune \
    && rm -rf /var/lib/apt/lists/*

# Install terminal emulators
RUN apt-get update && apt-get install -y \
    gnome-terminal \
    xterm \
    && rm -rf /var/lib/apt/lists/*

# Install Kitty (Alacritty skipped - not in Ubuntu 22.04 default repos)
RUN apt-get update && apt-get install -y kitty && \
    rm -rf /var/lib/apt/lists/*

# Install Gedit
RUN apt-get update && apt-get install -y gedit && \
    rm -rf /var/lib/apt/lists/*

# Cursor, Zed, and Helix skipped - installation issues or not in standard repos

# Install Kate (KDE editor)
RUN apt-get update && apt-get install -y kate && \
    rm -rf /var/lib/apt/lists/*

# Install Micro (modern terminal editor)
RUN curl https://getmic.ro | bash && \
    mv micro /usr/local/bin/

# Install IntelliJ IDEA Community Edition
RUN wget -q https://download.jetbrains.com/idea/ideaIC-2024.1.4.tar.gz && \
    tar xzf ideaIC-*.tar.gz -C /opt && \
    rm ideaIC-*.tar.gz && \
    ln -s /opt/idea-IC-*/bin/idea.sh /usr/local/bin/idea || echo "IntelliJ install skipped"

# Install PyCharm Community Edition
RUN wget -q https://download.jetbrains.com/python/pycharm-community-2024.1.4.tar.gz && \
    tar xzf pycharm-community-*.tar.gz -C /opt && \
    rm pycharm-community-*.tar.gz && \
    ln -s /opt/pycharm-community-*/bin/pycharm.sh /usr/local/bin/pycharm || echo "PyCharm install skipped"

# Create test workspace
RUN mkdir -p /workspace/test_project
WORKDIR /workspace

# Create entrypoint script
RUN echo '#!/bin/bash\n\
set -e\n\
\n\
# Start Xvfb in background\n\
Xvfb :99 -screen 0 1920x1080x24 -ac +extension GLX +render -noreset &\n\
XVFB_PID=$!\n\
\n\
# Wait for X server to be ready\n\
sleep 2\n\
\n\
# Execute the command\n\
exec "$@"' > /entrypoint.sh && \
    chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
CMD ["/bin/bash"]
