FROM node:20-slim

WORKDIR /sdk

# Copy package files
COPY sdks/javascript/package.json sdks/javascript/tsconfig.json sdks/javascript/jest.config.js ./

# Install dependencies
RUN npm install

# Copy SDK source
COPY sdks/javascript/ ./

# Build TypeScript
RUN npm run build

# Run tests
CMD ["npm", "test"]

