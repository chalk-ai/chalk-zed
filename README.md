# Chalk for Zed

This extension downloads and runs `chalk-lsp` for Python, `.chalk.sql`, and Chalk configuration buffers in worktrees containing `chalk.yml` or `chalk.yaml`. `chalk-lsp` is Chalk's language-intelligence process: it supplies diagnostics, navigation, and editor features. Dedicated Chalk SQL/YAML language definitions prevent the server from attaching to unrelated SQL or YAML files.

## Install

Until the extension is in Zed's registry, open the Extensions page, choose **Install Dev Extension**, and select this repository. New projects created by `chalk init` include:

```json
{
  "auto_install_extensions": { "chalk": true },
  "languages": { "Python": { "language_servers": ["chalk-lsp"] } }
}
```

The explicit server list makes Chalk the sole Python language server for that project. Zed may ask you to trust the worktree before applying project settings.

Each extension release pins a specific `chalk-lsp` release. On first use the extension downloads the pinned macOS/Linux x86-64 or ARM64 build, or Windows x86-64 build, and verifies its SHA-256 checksum.

When a newer extension release is available, click **Upgrade** for Chalk on Zed's Extensions page. Zed reloads the extension, and the extension replaces its managed `chalk-lsp` if the new release pins a different version. It does not perform background update checks of its own.

To see the installed `chalk-lsp` version, click Zed's language-server status icon and open **Chalk Language Server**. Zed displays the version reported by the running server next to its status.

Lookup, download, checksum, extraction, or replacement failures during an upgrade leave a valid existing server and its version metadata together in the existing `server` directory.

Open `zed: open log` to troubleshoot installation or startup.

## Development and publication

Install this repository as a dev extension to compile and exercise it. To ship a new `chalk-lsp`, update `CHALK_LSP_VERSION` and bump the version in both `extension.toml` and `Cargo.toml`.

Initial publication requires a PR adding the public `chalk-ai/chalk-zed` repository as the `chalk` submodule in `zed-industries/extensions`. After that PR is merged, pushing a bare semantic-version tag matching both manifests validates the WebAssembly build and opens the registry update PR automatically. The repository must have a `ZED_EXTENSION_PUBLISH_TOKEN` secret containing a personal access token with `repo` and `workflow` scopes; the publishing action creates or reuses that account's fork of `zed-industries/extensions`.
