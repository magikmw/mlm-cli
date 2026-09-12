# Signing

Every binary `mlm` publishes on its [GitHub Releases page](https://github.com/magikmw/mlm-cli/releases)
is signed with [minisign](https://jedisct1.github.io/minisign/). This is a
much simpler setup than [cargo-binstall's own just-in-time / keyless signing
scheme][binstall-signing]: one long-lived keypair, generated once, whose
public half is committed to this repo and whose private half lives only as a
CI secret.

[binstall-signing]: https://github.com/cargo-bins/cargo-binstall/blob/main/SIGNING.md

## What's signed

Each release artifact built by `cargo-dist` — the platform archives
(`mlm-<target>.tar.xz` / `.zip`) — gets a matching `<file>.sig` file uploaded
to the same GitHub Release. Signing happens in CI, after `cargo-dist` has
created the release and uploaded the archives (see
`.github/workflows/sign-artifacts.yml`), using a minisign private key stored
as the `MINISIGN_SECRET_KEY` repository secret.

## Algorithm and public key

The algorithm is `minisign` (Ed25519-based, via the `minisign`/`rsign2`
tooling). The public key is declared in `Cargo.toml`:

```toml
[package.metadata.binstall.signing]
algorithm = "minisign"
pubkey = "RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n"
```

That's the authoritative copy of the public key — if in doubt, trust the
`pubkey` value on the `main` branch of this repository over any copy quoted
elsewhere (including in this document).

## Verifying automatically: `cargo binstall`

If you install `mlm` with [`cargo binstall`](https://github.com/cargo-bins/cargo-binstall),
verification is automatic: binstall reads the `pubkey` from `Cargo.toml`,
downloads the `.sig` file alongside whichever archive it fetched, and
refuses to install if the signature doesn't check out. There's nothing you
need to do.

`cargo-dist` also publishes a `dist-manifest.json` with every release, which
recent `cargo-binstall` versions can consume directly for more precise
artifact resolution; this requires no extra configuration here either.

## Verifying manually: the `minisign` CLI

If you downloaded a release archive by hand (not via `cargo binstall`), you
can verify it yourself:

1. Install `minisign` (e.g. `dnf install minisign`, `apt install minisign`,
   `brew install minisign`, or `cargo install rsign2` as a pure-Rust
   alternative that speaks the same format).
2. Download both the archive and its matching `.sig` file from the
   [release page](https://github.com/magikmw/mlm-cli/releases), e.g.
   `mlm-x86_64-unknown-linux-gnu.tar.xz` and
   `mlm-x86_64-unknown-linux-gnu.tar.xz.sig`.
3. Verify, passing the public key from `Cargo.toml` directly on the command
   line:

   ```console
   minisign -Vm mlm-x86_64-unknown-linux-gnu.tar.xz \
     -P RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n
   ```

   A successful check prints `Signature and comment signature verified`. If
   it instead reports an invalid signature, do not run the binary — re-fetch
   the archive from the official release page, and if it still fails, open
   an issue.

## Key handling

The private key was generated with `minisign -G -W ...` (no password,
suitable for unattended CI signing) and is stored **only** as the
`MINISIGN_SECRET_KEY` GitHub Actions secret for this repository — it is not
committed anywhere. It is decoded from base64 and written to a temporary
file for the duration of the signing job, then discarded when the job ends.
If the key is ever suspected to be compromised, generate a fresh keypair,
update `pubkey` in `Cargo.toml`, and rotate the `MINISIGN_SECRET_KEY`
secret; older releases signed with the previous key remain verifiable
against the old public key, but new releases will need the new one.
