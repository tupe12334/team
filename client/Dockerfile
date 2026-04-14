# ─── Stage 1: Generate TypeScript proto bindings ──────────────────────────────
FROM node:20-alpine AS proto-gen

RUN apk add --no-cache protobuf-dev

WORKDIR /app

# Install root devDeps to get protoc-gen-ts_proto
COPY package.json pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

# Generate TypeScript from proto files
COPY proto/ ./proto/
RUN mkdir -p client/src/gen && \
    protoc \
      --plugin=protoc-gen-ts_proto=./node_modules/.bin/protoc-gen-ts_proto \
      --ts_proto_out=./client/src/gen \
      --ts_proto_opt=paths=source_relative,outputServices=grpc-js,esModuleInterop=true \
      -I ./proto \
      -I /usr/include \
      proto/agents.proto proto/daemon.proto proto/queue.proto proto/worker.proto

# ─── Stage 2: Build Next.js app ───────────────────────────────────────────────
FROM node:20-alpine AS builder

WORKDIR /app

COPY client/package.json client/pnpm-lock.yaml ./
RUN corepack enable && pnpm install --frozen-lockfile

COPY client/ ./
# Overwrite with freshly generated proto bindings (client/src/gen is gitignored)
COPY --from=proto-gen /app/client/src/gen/ ./src/gen/

RUN pnpm build

# ─── Stage 3: Minimal runtime using Next.js standalone output ─────────────────
FROM node:20-alpine AS runner

WORKDIR /app

ENV NODE_ENV=production

COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
COPY --from=builder /app/public ./public

EXPOSE 3000

CMD ["node", "server.js"]
