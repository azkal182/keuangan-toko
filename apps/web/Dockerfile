FROM node:26.13.1-bookworm-slim AS build
WORKDIR /src
RUN npm install --global pnpm@12.21.0
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/package.json
COPY packages/api-client/package.json packages/api-client/package.json
COPY packages/api-client packages/api-client
RUN pnpm install --frozen-lockfile
COPY apps/web apps/web
RUN pnpm --filter @toko/web build

FROM nginx:1.29.3-alpine
COPY infra/docker/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=build /src/apps/web/dist /usr/share/nginx/html
EXPOSE 80
