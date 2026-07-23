# The Docker rig: a Linux machine for the anvil's native leaderboards.
#
# On macOS and Windows, Docker runs containers inside a Linux virtual
# machine, so scores measured here really are Linux scores - a different
# operating system and (on Apple hardware) a different board than the host.
# The harness is compiled inside the image; `razor bench --rig <id>` runs it
# through `docker run` (the rig's registered runner command), so the
# measurement never touches the host toolchain.
#
# Build (from the repository root):
#   docker build -t satoshis-anvil-rig -f anvil/rig/docker/Dockerfile .
# Or just run ./docker-rig.sh, which builds, registers, and benches.

FROM rust:1.92-slim

WORKDIR /razor

# The workspace manifest lists every member crate, so the whole tree is
# copied (.dockerignore drops target/, lean/, site/, and the datasets) and
# only the harness package is built.
COPY . .

RUN cargo build --release -p anvil-harness

# External lanes (anvil/lanes/*) bring their own build scripts - compile
# them for this container so they compete on the Linux boards too.
RUN set -e; for b in anvil/lanes/*/build.sh; do [ -f "$b" ] && sh "$b"; done || true

ENTRYPOINT ["/razor/target/release/anvil-harness"]
