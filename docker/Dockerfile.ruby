# Dockerfile for testing Ruby SDK examples
FROM ruby:3.4-slim

WORKDIR /app

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Copy Ruby SDK
COPY sdks/ruby /app/sdks/ruby

# Install dependencies (if any)
WORKDIR /app/sdks/ruby

# Run examples
CMD ["sh", "-c", "ruby examples/hello_world.rb && ruby examples/sprite_demo.rb && ruby examples/3d_scene.rb"]

