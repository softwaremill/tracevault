FROM node:22-slim AS build
WORKDIR /app
RUN npm install -g pnpm
COPY package.json pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm build

FROM node:22-slim
WORKDIR /app
COPY --from=build /app/build build/
COPY --from=build /app/package.json .
ENV NODE_ENV=production
ENV WEB_PORT=4000
ENV WEB_HOST=0.0.0.0
EXPOSE 4000
CMD ["node", "build"]
