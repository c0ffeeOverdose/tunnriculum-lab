FROM php:8.3-apache

RUN docker-php-ext-install mysqli pdo pdo_mysql

# sshd so players can use sshuttle/SSH -D after finding creds (pivot lesson);
# python3 (+ python symlink) because sshuttle executes python on the pivot host
RUN apt-get update && apt-get install -y --no-install-recommends openssh-server iproute2 iputils-ping curl netcat-openbsd sshpass socat python3 \
    && ln -s /usr/bin/python3 /usr/local/bin/python \
    && rm -rf /var/lib/apt/lists/* \
    && mkdir -p /run/sshd

# ligolo-ng agent (userspace, runs as www-data — no root needed on the pivot)
ADD https://github.com/nicocha30/ligolo-ng/releases/download/v0.9.1/ligolo-ng_agent_0.9.1_linux_amd64.tar.gz /tmp/ligolo.tgz
RUN tar xzf /tmp/ligolo.tgz -C /tmp && \
    mkdir -p /opt/tools && install -m 755 /tmp/agent /opt/tools/agent && \
    rm -rf /tmp/ligolo.tgz /tmp/agent /tmp/LICENSE /tmp/README.md

COPY src/ /var/www/html/
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh \
    && mkdir -p /var/www/html/uploads \
    && chown -R www-data:www-data /var/www/html/uploads

ENTRYPOINT ["/entrypoint.sh"]
