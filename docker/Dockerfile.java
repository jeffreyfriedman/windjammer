FROM maven:3.9-eclipse-temurin-17

WORKDIR /sdk

# Copy pom.xml
COPY sdks/java/pom.xml ./

# Download dependencies
RUN mvn dependency:go-offline || true

# Copy SDK source
COPY sdks/java/ ./

# Build and test
CMD ["mvn", "clean", "test"]

