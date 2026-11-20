FROM docker.io/bitcoin/bitcoin:31.1@sha256:da25cedc66b1daefff9f412ee196c901a899c3fa68a33b20849c3e08b5c40d63 AS regtest-tools

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends bash python3 \
    && rm -rf /var/lib/apt/lists/*
COPY --chmod=755 scripts/init-regtest.py scripts/check-regtest.py /usr/local/bin/
ENTRYPOINT []
CMD ["/usr/local/bin/init-regtest.py"]

FROM docker.io/mempool/backend:v3.3.1@sha256:358c0a517c8dcf26e7f5c02447de5bab33ec7e3fa6318685cf8012ce36098e3a AS mempool-source

FROM docker.io/library/node:24.13.0-bookworm-slim@sha256:4660b1ca8b28d6d1906fd644abe34b2ed81d15434d26d845ef0aced307cf4b6f AS mempool-backend

WORKDIR /backend
COPY --from=mempool-source --chown=node:node /backend/package ./package
COPY --from=mempool-source --chown=node:node /backend/mempool-config.json /backend/start.sh ./
RUN chown node:node /backend
USER node
EXPOSE 8999
CMD ["/backend/start.sh"]
