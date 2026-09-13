# Security Policy

`mlm` is a personal-use CLI tool. This document explains how to report
security issues and how to verify that a release binary is authentic.

## Reporting a vulnerability

Please report security issues privately by emailing
**magikmw@michalwalczak.eu**. Do not open a public issue or disclose the
problem publicly until it has been addressed.

There is no bug bounty program — reports are appreciated, but no monetary
reward is offered.

## Supported versions

Only the latest released version is supported. Please update to the latest
release before reporting an issue, and expect fixes to land only in new
releases rather than backports to older ones.

## Binary signing / verification

Release binaries are signed with [minisign](https://jedisct1.github.io/minisign/).
The public key is committed in `Cargo.toml`
(`[package.metadata.binstall.signing]`):

```
RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n
```

If you install with `cargo binstall`, verification happens automatically
using that key. To verify a manually downloaded archive, fetch its matching
`.sig` file from the [release page](https://github.com/magikmw/mlm-cli/releases)
and run:

```console
minisign -Vm mlm-<target>.tar.xz -P RWTNO3Iugq4ICzcJhLwnwtOT1T0evn7RYI0951dfE+IkEyRykSvv0V/n
```

See `docs/SIGNING.md` for the full signing setup, including CI details and
key-rotation procedure.
