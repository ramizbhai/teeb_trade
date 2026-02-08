# Teeb Trade - Deployment Guide

This guide explains how to deploy Teeb Trade to your production server (172.86.117.232) using Podman.

## Prerequisites
- Server with **Podman** and **Git** installed.
- **podman-compose** Python script installed (or docker-compose compatible).

## Deployment Steps

### 1. Push Code (Local Machine)
Ensure all changes are committed and pushed to the repository.
```bash
git add .
git commit -m "Deployment setup"
git push origin main
```

### 2. Pull Code (Server)
SSH into your server and navigate to the project directory.
```bash
cd teeb_trade
git pull origin main
```

### 3. Start Application (Server)
Build and start the containers in detached mode.
```bash
podman-compose up -d --build
```
*Note: This will build the Rust backend and Node frontend images. Initial build may take a few minutes.*

## Verify Deployment
- **Backend API**: `http://172.86.117.232:3000`
- **Frontend App**: `http://172.86.117.232:5173`

## Logs & Maintenance
- **View Logs**: `podman-compose logs -f`
- **Stop**: `podman-compose down`
- **Restart**: `podman-compose restart`
- **Data Persistence**: Signals are saved to `backend/history.json` on the host machine.
