# Releases

Official installers are built and published by [Release](../.github/workflows/release.yml) on GitHub-hosted runners. Local builds remain useful for development, but are not official releases.

## Publish a version

1. Update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` to the same version.
2. Add `docs/releases/vX.Y.Z.md` with release notes and merge the changes into `main`.
3. Run **Actions → Release → Run workflow**, selecting `main`.

The workflow checks version consistency, runs the CI suite, and builds Linux x86_64, Windows x86_64, macOS Apple Silicon, and macOS Intel installers. It creates SHA-256 checksums, attests the downloads, verifies their provenance, then creates and publishes the release. It creates the version tag at the exact tested commit; do not create release tags locally.

No local installers, personal access tokens, or long-lived signing secrets are used by this pipeline. Build jobs have read-only repository permissions. Only the publication job can write releases or attestations, and it downloads artifacts from its own workflow run. Dependencies use npm and Cargo lockfiles; top-level actions are pinned to commit hashes.

## Verify a download

With a recent GitHub CLI:

```bash
gh attestation verify ./YOUR_DOWNLOADED_FILE --repo Shin-Aska/Threadline \
  --signer-workflow Shin-Aska/Threadline/.github/workflows/release.yml \
  --source-ref refs/heads/main --deny-self-hosted-runners
```

For an exact release commit, also pass `--source-digest COMMIT_SHA`. The CLI checks the artifact digest and the workflow’s signed identity. Checksums alone detect changes but do not prove who built a file.

The first release is not Windows publisher-signed or Apple-notarized. GitHub attestations establish CI provenance; they do not supply those platform identities. Do not claim reproducible builds or a specific SLSA level without separately verifying those properties.

## Repository controls

Release immutability must be enabled in GitHub; the workflow refuses to publish without it. Release tags prohibit updates/deletion. The `release` environment is restricted to `main`. These are GitHub settings, not portable Git configuration.

GitHub does not allow its built-in Actions integration as a tag-creation bypass actor for this personal repository. Tag creation is therefore not restricted to that integration; the workflow rejects existing version tags, and verified provenance distinguishes CI artifacts from manual uploads.

Published immutable assets cannot be replaced. Fix a released version by making a new version. If publication fails after creating a draft, inspect and delete that unpublished draft and its unused tag through the repository’s administrative recovery process before retrying; never disable immutability to replace a published release.

The controls protect the normal publishing path. A repository administrator can change workflows, permissions, or rules. GitHub does not provide an unconditional “no administrator can manually publish” switch; consumers should verify provenance, not trust an asset’s filename alone.
