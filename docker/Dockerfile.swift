# Dockerfile for testing Swift SDK examples
FROM swift:6.2

WORKDIR /app

# Copy Swift SDK
COPY sdks/swift /app/sdks/swift

# Run examples
WORKDIR /app/sdks/swift
CMD ["sh", "-c", "swift examples/HelloWorld.swift && swift examples/SpriteDemo.swift && swift examples/3DScene.swift"]

