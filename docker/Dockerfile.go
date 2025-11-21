FROM golang:1.25-alpine

WORKDIR /sdk

# Copy go module files
COPY sdks/go/go.mod sdks/go/go.sum* ./

# Download dependencies
RUN go mod download || true

# Copy SDK source
COPY sdks/go/ ./

# Build
RUN go build -v ./...

# Run tests
CMD ["go", "test", "-v", "./..."]

