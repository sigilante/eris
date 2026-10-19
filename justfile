# Eris orchestrator — two independent artifacts (serf: cargo, king: zig) + glue.
# The king and serf are separate processes, so nothing links across the C/Rust
# boundary; we build each with its native toolchain and wire them with --serf-bin.
# See docs/build.md. Run `just` for the recipe list.

vere    := "third_party/vere"
serfdir := "serf"
profile := "release"                                   # serf build profile: release|debug
cargoflags := if profile == "release" { "--release" } else { "" }
ship := "zod"                                          # default fake-ship for `dev`
pier := "pier-" + ship

_default:
    @just --list

# Apply the patch series onto the pinned submodules (idempotent).
apply:
    ./patches/apply.sh

# Build the mars/serf (Rust, nightly pinned) -> serf/target/<profile>/eris-serf
serf *ARGS: apply
    cd {{serfdir}} && cargo build {{cargoflags}} {{ARGS}}

# Build the urth/king: stock vere via Zig, named `eris`, ReleaseFast.
# No vere patch — mars.c stays compiled but unused; we drive via --serf-bin.
# -> third_party/vere/zig-out/<triple>/eris   (first build is slow: builds ext/*)
king:
    cd {{vere}} && zig build -Drelease -Dbinary-name=eris

# Build both artifacts.
build: serf king

# Print resolved binary paths.
serf-bin:
    @echo "$(pwd)/{{serfdir}}/target/{{profile}}/eris-serf"
king-bin:
    @ls {{vere}}/zig-out/*/eris 2>/dev/null || echo "(king not built; run: just king)"

# Boot a fake ship with the king driving the Eris serf.
# Requires a pill:  just dev PILL=/path/to/brass.pill  [ship=bus]
# (Will not boot until the serf implements the newt seam — see docs/seam-contract.md.)
dev PILL="": build
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "{{PILL}}" ]; then
      echo "usage: just dev PILL=/path/to/pill [ship=NAME]" >&2; exit 2
    fi
    king="$(ls {{vere}}/zig-out/*/eris | head -1)"
    serf="$(pwd)/{{serfdir}}/target/{{profile}}/eris-serf"
    echo ">>> king:  $king"
    echo ">>> serf:  $serf  (--serf-bin)"
    echo ">>> pier:  {{pier}}   ship: {{ship}}"
    exec "$king" --fake {{ship}} --serf-bin "$serf" \
         --bootstrap "{{PILL}}" --pier "{{pier}}"

# Remove the dev pier (DESTROYS the fake ship's state).
clean-pier:
    rm -rf "{{pier}}"
