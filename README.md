<div align="center">

[![License][license-badge]](LICENSE)
[![CI Status][ci-badge]][ci]
[![Conda Platform][conda-badge]][conda-url]
[![Conda Downloads][conda-downloads-badge]][conda-url]
[![Project Chat][chat-badge]][chat-url]
[![Pixi Badge][pixi-badge]][pixi-url]

[license-badge]: https://img.shields.io/github/license/pavelzw/pixi-inject?style=flat-square
[ci-badge]: https://img.shields.io/github/actions/workflow/status/pavelzw/pixi-inject/ci.yml?style=flat-square&branch=main
[ci]: https://github.com/pavelzw/pixi-inject/actions/
[conda-badge]: https://img.shields.io/conda/vn/conda-forge/pixi-inject?style=flat-square
[conda-downloads-badge]: https://img.shields.io/conda/dn/conda-forge/pixi-inject?style=flat-square
[conda-url]: https://prefix.dev/channels/conda-forge/packages/pixi-inject
[chat-badge]: https://img.shields.io/discord/1082332781146800168.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2&style=flat-square
[chat-url]: https://discord.gg/kKV8ZxyzY4
[pixi-badge]: https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/prefix-dev/pixi/main/assets/badge/v0.json&style=flat-square
[pixi-url]: https://pixi.sh

</div>

# pixi-inject

This is a simple executable that injects a conda package into an existing pixi environment.

```bash
pixi-inject --environment default --package my-package-0.1.0-py313h8aa417a_0.conda
```

You can also specify a custom conda prefix to inject the package into.

```bash
pixi-inject --prefix /path/to/conda/env --package my-package-0.1.0-py313h8aa417a_0.conda
```
