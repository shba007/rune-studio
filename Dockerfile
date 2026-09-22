FROM oven/bun:1-alpine AS builder

WORKDIR /app

COPY package.json bun.lock ./

ENV VUE_PUBLIC_SITE_URL=$SITE_URL

RUN bun install --frozen-lockfile

COPY . .

RUN bun run build

FROM oven/bun:1-alpine AS runner

ARG VERSION
ARG BUILD_TIME

COPY ./nginx.conf /etc/nginx/nginx.conf

COPY --from=builder /app/dist /usr/share/nginx/html

ENV NODE_ENV=production
ENV VUE_APP_VERSION=$VERSION
ENV VUE_APP_BUILD_TIME=$BUILD_TIME

EXPOSE 8080