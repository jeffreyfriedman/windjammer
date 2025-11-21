# Dockerfile for testing Java SDK examples
FROM maven:3.9-eclipse-temurin-17

WORKDIR /app

# Copy Java SDK
COPY sdks/java /app/sdks/java

# Build SDK
WORKDIR /app/sdks/java
RUN mvn clean package -DskipTests

# Run examples
CMD ["sh", "-c", "mvn exec:java -Dexec.mainClass=dev.windjammer.examples.HelloWorld && mvn exec:java -Dexec.mainClass=dev.windjammer.examples.SpriteDemo && mvn exec:java -Dexec.mainClass=dev.windjammer.examples.Scene3D"]
