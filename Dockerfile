# Use a Rust base image
FROM rust:latest as builder

# Set the working directory
WORKDIR /usr/src/gust

# Copy the Rust project files to the working directory
COPY . .

# Build the Rust executable
RUN cargo build --release

# Create a new image without the build dependencies
FROM debian:bullseye-slim

# Set the working directory
WORKDIR /usr/src/gust

# Copy the built executable from the builder stage
COPY --from=builder /usr/src/gust/target/release/gust .

# Copy the .env file into the Docker image
COPY .env .

# Run the Rust executable command
CMD ["./gust", "http"]
