$ErrorActionPreference = "Stop"

Write-Host "Building images for linux/amd64..."
docker compose build --no-cache

Write-Host "Saving images to go-news-images.tar..."
docker save -o go-news-images.tar go-news-frontend:latest go-news-backend:latest

Write-Host "Done! Images exported to go-news-images.tar."
