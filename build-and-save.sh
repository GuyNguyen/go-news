#!/bin/bash
set -e

# Build the images from docker-compose without cache
docker compose build --no-cache

# Save the built images to a tar file
docker save -o go-news-images.tar go-news-frontend go-news-backend
