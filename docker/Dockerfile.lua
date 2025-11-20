# Dockerfile for testing Lua SDK examples
FROM alpine:3.19

WORKDIR /app

# Install Lua
RUN apk add --no-cache \
    lua5.4 \
    lua5.4-dev \
    luarocks \
    git \
    gcc \
    musl-dev

# Copy Lua SDK
COPY sdks/lua /app/sdks/lua

# Install SDK (if it has dependencies)
WORKDIR /app/sdks/lua

# Run examples
CMD ["sh", "-c", "lua examples/hello_world.lua && lua examples/sprite_demo.lua && lua examples/3d_scene.lua"]

