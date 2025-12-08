FROM linuxserver/ffmpeg:latest
WORKDIR /workspace
ENV TERM=xterm-256color
ENV LIBVA_DRIVERS_PATH=/usr/local/lib/x86_64-linux-gnu/dri
# NVIDIA runtime hint: set by nvidia-container-toolkit on host
ENV NVIDIA_DRIVER_CAPABILITIES=compute,utility,video
ENV NVIDIA_VISIBLE_DEVICES=all

# Install SSH server (password-based only)
RUN apt-get update && apt-get install -y openssh-server && rm -rf /var/lib/apt/lists/*
RUN mkdir -p /var/run/sshd

# Copy prebuilt ffdash binary (built on host before docker build)
COPY target/release/ffdash /usr/local/bin/ffdash
RUN chmod +x /usr/local/bin/ffdash

COPY docker/entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
