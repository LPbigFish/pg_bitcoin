FROM docker.io/bitcoin/bitcoin:31.1-alpine@sha256:6b7ab829b25388ef523c164f027e69bdac341e6b85b6c4794b9fd8839dcd91f1 AS regtest-tools

USER root
RUN apk add --no-cache bash jq xxd
COPY --chmod=755 scripts/init-regtest.sh scripts/check-regtest.sh /usr/local/bin/
ENTRYPOINT []
CMD ["/usr/local/bin/init-regtest.sh"]

FROM docker.io/mempool/backend:v3.3.1@sha256:358c0a517c8dcf26e7f5c02447de5bab33ec7e3fa6318685cf8012ce36098e3a AS mempool-source

FROM docker.io/library/node:24.13.0-bookworm-slim@sha256:4660b1ca8b28d6d1906fd644abe34b2ed81d15434d26d845ef0aced307cf4b6f AS mempool-backend

WORKDIR /backend
COPY --from=mempool-source --chown=node:node /backend/package ./package
COPY --from=mempool-source --chown=node:node /backend/mempool-config.json /backend/start.sh ./
RUN chown node:node /backend
USER node
EXPOSE 8999
CMD ["/backend/start.sh"]
