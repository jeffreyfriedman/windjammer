FROM gcc:13

WORKDIR /sdk

# Install CMake
RUN apt-get update && apt-get install -y cmake && rm -rf /var/lib/apt/lists/*

# Copy SDK source
COPY sdks/cpp/ ./

# Build
RUN mkdir -p build && cd build && cmake .. && make

# Run tests (if we add them)
CMD ["echo", "C++ SDK built successfully"]

