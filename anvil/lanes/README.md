# External lanes

A lane that is not compiled into the harness. Drop a directory here -
`anvil/lanes/<impl-name>/lane.json` - and the harness picks it up by name:
no harness rebuild, no registry change, any implementation language.

```json
{
  "challenge": "crc64",
  "native": "crc64-byte-c",
  "wasm": "crc64_byte_c.wasm",
  "language": "C",
  "arch": "aarch64",
  "note": "free text"
}
```

`native` and `wasm` are paths relative to the lane directory; either may be
omitted (a GPU- or ISA-specific lane may be native-only, a portable one
wasm-only). The harness reports a missing artifact as "not measurable",
the same way the GPU lane behaves on a GPU-less machine.

`language` is informational - it says what the artifact is written in and
shows up in notes, nothing gates on it. `arch` is enforced: set it
(`aarch64`, `x86_64`, ...) when the native artifact is hand-written
assembly or uses ISA-specific intrinsics, and the harness will only run
that binary on a matching machine - elsewhere the lane falls back to its
wasm build for the differential check and reports the native tier as not
measurable. Omit `arch` for portable code.

## The native protocol

The native artifact is any executable that implements two subcommands:

- `native --seed S --iters I --repeats R` - run the pinned input stream
  through the implementation R times (after one warm-up), and print one
  JSON line: `{"tier":"native","arch":"<arch>","impl":"<name>","seed":S,
  "iters":I,"ns":<median total ns>,"ns_per_op":<ns/I>,"checksum":<sum>}`.
- `many --seed S --iters I` - write the implementation's output for each
  input word to stdout, little-endian u64s, in stream order. The harness
  uses this for the differential check against the challenge's reference.

The input stream is the anvil's pinned generator: `state = seed | 1`, then
per word `state ^= state<<13; state ^= state>>7; state ^= state<<17`,
mapped through the challenge's input map (see the harness's CHALLENGES
table; most are the identity). The checksum is the wrapping sum of all
outputs. `anvil/lanes/crc64-byte-c/crc64_byte_c.c` is a complete example.

## The wasm artifact

Same contract as a built-in lane's wasm build: export
`bench(seed: u64, iters: u64) -> u64` (checksum over the generated stream)
and `solve_one(x: u64) -> u64`. Any toolchain that targets
wasm32-unknown-unknown works.

## What does NOT change

Admission still requires the Lean refinement proof against the challenge
spec (`razor sorry` + `razor submit`), and every score still passes the
differential check on the exact benchmark stream first. The external-lane
protocol changes who compiles the code, not what is trusted: the model in
Lean is hand-transliterated from the source whatever the language, and the
differential check ties the artifact to it on the scored inputs.
