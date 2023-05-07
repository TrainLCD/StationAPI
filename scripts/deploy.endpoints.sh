#!/bin/bash

python -m grpc_tools.protoc \
    --include_imports \
    --include_source_info \
    --proto_path=. \
    --descriptor_set_out=api_descriptor.pb \
    ./proto/stationapi.proto
gcloud endpoints services deploy api_descriptor.pb api_config.yaml