FROM debian:12-slim

RUN apt-get update && apt-get install -y --no-install-recommends openssh-server iproute2 \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /run/sshd

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
