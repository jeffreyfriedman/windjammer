# Dockerfile for testing C++ SDK examples
FROM gcc:16

WORKDIR /app

# Install CMake
RUN apt-get update && apt-get install -y \
    cmake \
    ninja-build \
    git \
    && rm -rf /var/lib/apt/lists/*

# Copy C++ SDK
COPY sdks/cpp /app/sdks/cpp

# Build examples
WORKDIR /app/sdks/cpp
RUN mkdir -p build && cd build && \
    cmake .. -GNinja && \
    ninja

# Run tests
CMD ["sh", "-c", "./build/examples/hello_world && ./build/examples/math_demo && ./build/examples/sprite_demo && ./build/examples/3d_scene"]
