# === Build stage ===
FROM rust:1-slim-bookworm AS build

WORKDIR /app

# Copy the project files
COPY . .

# RUN apt-get update && apt-get install -y vim lsof net-tools

# Build the project in release mode
RUN cargo build --release

# === Runtime stage ===
FROM debian:bookworm-slim

WORKDIR /app

# Install required shared libraries
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy the compiled binary from the build stage
COPY --from=build /app/target/release/merkle-tree-api ./

# Define port and entrypoint
ENV PORT=8080
EXPOSE $PORT
CMD ["./merkle-tree-api"]
