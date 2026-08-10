FROM oven/bun:1.3.14-alpine AS dependencies
WORKDIR /app
COPY package.json ./
COPY console/package.json console/package.json
RUN bun install

FROM dependencies AS build
COPY . .
RUN bun --cwd console build

FROM oven/bun:1.3.14-alpine AS runtime
WORKDIR /app
ENV NODE_ENV=production
RUN addgroup --system glance && adduser --system --ingroup glance glance
COPY --from=build /app ./
USER glance
EXPOSE 3000
CMD ["bun", "--cwd", "console", "start"]
