#!/bin/bash
set -e

echo "Building images for linux/amd64..."
docker compose build --no-cache

echo "Saving images to go-news-images.tar..."
docker save -o go-news-images.tar go-news-frontend:latest go-news-backend:latest

echo "Done! Images exported to go-news-images.tar."
