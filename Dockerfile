# Build the server
FROM rust:1.76.0 as builder
# Copy website server files
WORKDIR /home/website/server
COPY . .
RUN cargo build --release
RUN echo $PWD

FROM ubuntu:24.04
COPY --from=builder /home/website/server/target/release/backend /home/website/server/target/release/backend
COPY --from=builder /home/website/server/templates /home/website/server/target/release/templates

# Install dependencies
RUN apt-get update && \
    apt-get install -y curl tree hugo git build-essential nodejs npm

# Set working directory
WORKDIR /home/website

# Clone the theme repository
RUN git clone https://github.com/fscs/website-theme

# Build the website
WORKDIR /home/website/website-theme/demo
RUN ls
RUN hugo

# index using pagefind
RUN npx -y pagefind --site public

# Copy the website to the server
WORKDIR /home/website/server/target/release
RUN cp -r /home/website/website-theme/demo/public/ static


# Set the entrypoint to run the server
EXPOSE 8080

CMD ["./backend --host 0.0.0.0"]
