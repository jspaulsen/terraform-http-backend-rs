#!/usr/bin/env bash

VERSION=$(source version.sh)
PREFIX="jspaulsen/"
TAG="terraform-http-backend"
TARGET="base"
PUBLISH=false

BRANCH=$(git symbolic-ref --short -q HEAD)
INCLUDE_LATEST=""

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
VTAG="${TAG}:v${VERSION}"
IMAGE="${PREFIX}${VTAG}"


if [[ $BRANCH == "main" ]]; then
    INCLUDE_LATEST="--tag ${PREFIX}${TAG}"
fi


docker build \
    --target ${TARGET} \
    --tag $IMAGE \
    --tag $TAG \
    $INCLUDE_LATEST \
    -f Dockerfile \
    .

# Publish
if [ "$PUBLISH" = true ]; then
    echo "Publishing image ${IMAGE}"
    docker image push $IMAGE
fi
