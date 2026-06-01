# Homebrew publishing (custom tap)

This repo is set up for a **custom Homebrew tap** flow, which is the easiest path during beta.

## 1) Create the tap repository

Create a repository named:

- `homebrew-tap`

Under your account, this becomes:

- `amaduswaray/homebrew-tap`

Homebrew tap name will be:

- `amaduswaray/tap`

## 2) Add formula to tap

This repository includes a reference formula at:

- `packaging/homebrew/shellql.rb`

In your tap repo, the formula must live at:

- `Formula/shellql.rb`

## 3) Enable automated sync from releases

A workflow is provided:

- `.github/workflows/homebrew-tap-sync.yml`

It runs when:
- a GitHub release is published, or
- manually via workflow dispatch

What it does:
- reads release tag (for example `v0.1.2-beta`)
- downloads release binaries from that tag (`shql-linux-x86_64`, `shql-macos-arm64`)
- computes platform `sha256` values
- regenerates `Formula/shellql.rb` to install prebuilt binaries
- commits/pushes it to your tap repo

### Required secret

In this repo, add:

- `HOMEBREW_TAP_GITHUB_TOKEN`

Token needs write access to the tap repository (`amaduswaray/homebrew-tap`).

## 4) User install command

Once tap has the formula:

```bash
brew tap amaduswaray/tap
brew install shellql
```

## 5) Manual sync (optional)

You can run **Actions → Homebrew Tap Sync → Run workflow** and pass:
- `tag` (for example `v0.1.2-beta`)
- optional `tap_repo` if different from default

## Notes

- This is ideal for beta (`v0.1.x-beta`) and avoids `homebrew/core` restrictions.
- Because the formula installs **prebuilt binaries**, users avoid heavy build dependencies (`rust`, `llvm`, etc.) during `brew install`.
- If you later target `homebrew/core`, you’ll need to meet stricter acceptance criteria.
