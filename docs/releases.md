# Releases

Official installers are built and published by [Release](../.github/workflows/release.yml) on GitHub-hosted runners. Local builds remain useful for development, but are not official releases.

## Publish a version

1. Update `package.json`, `package-lock.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, and `src-tauri/tauri.conf.json` to the same version.
2. Add `docs/releases/vX.Y.Z.md` with release notes and merge the changes into `main`.
3. Tag that commit as `vX.Y.Z` and push the tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.

Like the release workflows in [Amagon](https://github.com/Shin-Aska/amagon-html-editor/blob/main/.github/workflows/release.yml) and [DOSBox Staging Replacer](https://github.com/Shin-Aska/DosboxStagingReplacerForGOGGalaxy/blob/main/.github/workflows/release.yml), a version tag triggers a build and release. Threadline checks that the tag matches the package version and points to a commit on `main`, runs the CI suite, and builds Linux x86_64, Windows x86_64, macOS Apple Silicon, and macOS Intel installers. It creates SHA-256 checksums, attests the downloads, verifies their provenance, then creates and publishes the release. Manual dispatch is also supported on an existing version tag.

No local installers, personal access tokens, or long-lived signing secrets are used by this pipeline. Build jobs have read-only repository permissions. Only the publication job can write releases or attestations, and it downloads artifacts from its own workflow run. Dependencies use npm and Cargo lockfiles; top-level actions are pinned to commit hashes.

## Verify a download

With a recent GitHub CLI:

```bash
gh attestation verify ./YOUR_DOWNLOADED_FILE --repo Shin-Aska/Threadline \
  --signer-workflow Shin-Aska/Threadline/.github/workflows/release.yml \
  --source-ref refs/tags/v0.1.0 --deny-self-hosted-runners
```

For an exact release commit, also pass `--source-digest COMMIT_SHA`. The CLI checks the artifact digest and the workflow’s signed identity. Checksums alone detect changes but do not prove who built a file.

The first release is not Windows publisher-signed or Apple-notarized. GitHub attestations establish CI provenance; they do not supply those platform identities. Do not claim reproducible builds or a specific SLSA level without separately verifying those properties.

## Repository controls

Release immutability is enabled in GitHub. The normal workflow token cannot read that administrator-only setting, so the pipeline verifies the published release’s immutable flag instead. Release tags prohibit updates/deletion. The `release` environment permits version tags; the preparation job verifies their commits belong to `main`. Actions must be pinned to full commit hashes. These are GitHub settings, not portable Git configuration.

Maintainers create version tags to start CI. Verified provenance distinguishes CI artifacts from manual uploads; no locally built files enter the release pipeline.

Published immutable assets cannot be replaced. Fix a released version by making a new version. If publication fails after creating a draft, inspect and delete that unpublished draft before retrying the same tag. If source changes are required, use a new version tag; never move a protected tag or disable immutability to replace a published release.

The controls protect the normal publishing path. A repository administrator can change workflows, permissions, or rules. GitHub does not provide an unconditional “no administrator can manually publish” switch; consumers should verify provenance, not trust an asset’s filename alone.
