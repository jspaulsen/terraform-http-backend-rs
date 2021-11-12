#!/usr/bin/env bash

VERSION=$(source version.sh)
PREFIX="jspaulsen/"
TAG="terraform-http-backend"
TARGET="base"
PUBLISH=false

POSITIONAL=()

while [[ $# -gt 0 ]]; do
  key="$1"

  case $key in
    --target)
      TARGET="$2"
      shift # past argument
      shift # past value
      ;;
    --tag)
      TAG="$2"
      shift # past argument
      shift # past value
      ;;
    --publish)
      PUBLISH=true
      shift # past argument
      ;;
    *)    # unknown option
      POSITIONAL+=("$1") # save it in an array for later
      shift # past argument
      ;;
  esac
done


ARCH="$(dpkg --print-architecture)"
VTAG="${TAG}:v${VERSION}-${ARCH}"
IMAGE="${PREFIX}${VTAG}"

docker build \
    --target ${TARGET} \
    --tag $IMAGE \
    --tag $TAG \
    -f Dockerfile \
    .



# Publish
if [ "$PUBLISH" = true ]; then
    echo "Publishing image ${IMAGE}"
    docker image push $IMAGE
fi
