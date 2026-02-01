# Container image settings
IMAGE_NAME := "ctrlok/anker767-webserver"

# Build arm64 image
build-arm64 version:
    docker buildx build \
        --platform linux/arm64 \
        --tag {{IMAGE_NAME}}:{{version}}-arm64 \
        --load \
        .

# Build amd64 image
build-amd64 version:
    docker buildx build \
        --platform linux/amd64 \
        --tag {{IMAGE_NAME}}:{{version}}-amd64 \
        --load \
        .

# Build both architectures and create manifest
build version:
    docker buildx build \
        --platform linux/arm64,linux/amd64 \
        --tag {{IMAGE_NAME}}:{{version}} \
        --tag {{IMAGE_NAME}}:latest \
        .

# Build and push to registry
push version:
    docker buildx build \
        --platform linux/arm64,linux/amd64 \
        --tag {{IMAGE_NAME}}:{{version}} \
        --tag {{IMAGE_NAME}}:latest \
        --push \
        .

# Create buildx builder if not exists
setup-buildx:
    docker buildx create --name multiarch --use || docker buildx use multiarch
    docker buildx inspect --bootstrap
