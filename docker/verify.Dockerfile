# The throwaway verification container for the hosted registry.
#
# A submission is untrusted code: Lean elaboration can run programs. The
# hosted verifier therefore runs every kernel check in a fresh container
# from this image: the Lean package is mounted read-only and copied inside,
# the network namespace is empty, and the container dies with the check.
#
#   docker build -t razor-verify -f docker/verify.Dockerfile .
#   RAZOR_VERIFY_DOCKER=razor-verify razor serve
FROM ubuntu:24.04
RUN apt-get update && apt-get install -y --no-install-recommends \
      curl ca-certificates git bash coreutils \
    && rm -rf /var/lib/apt/lists/*
RUN curl -sSf https://elan.lean-lang.org/elan-init.sh | sh -s -- -y --default-toolchain none
ENV PATH=/root/.elan/bin:$PATH
# Preinstall the pinned toolchain so a check never touches the network.
COPY lean/lean-toolchain /tmp/lean-toolchain
RUN elan toolchain install "$(cat /tmp/lean-toolchain)" \
    && elan default "$(cat /tmp/lean-toolchain)"
