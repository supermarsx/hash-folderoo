# Scoop packaging config

Place any additional packaging-related configuration or scripts here. This folder contains example manifest(s) used to publish a Scoop bucket entry for `hash-folderoo`.

- `manifest.json` is a template to publish into a Scoop bucket and uses placeholders for `{{version}}` and `SHA256_PLACEHOLDER` which should be replaced by the release script/CI.

Scoop users can add a custom bucket or the project can provide an official bucket that points to releases on GitHub.
