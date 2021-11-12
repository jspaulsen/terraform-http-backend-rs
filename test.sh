#!/usr/bin/env bash

TAG="terraform-http-backend-tests"

source build.sh --target build --tag $TAG

docker run $TAG cargo test --release
