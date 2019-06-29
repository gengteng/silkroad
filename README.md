# SilkRoad

## Introduction

A full-featured registry server for Cargo. 

## Status

[WIP] Most features have not been implemented yet.

## RoadMap

- [ ] Serve a index repository and crates
    - [x] HTTP server
        - [x] The Dumb Protocol
        - [x] The Smart Protocol(except git-receive-pack)
    - [ ] Git server
    - [ ] No dependency on the `git` command
- [ ] Periodic update
    - [ ] Downloader
    - [ ] Timer
- [ ] Server Migration
    - [ ] Package
    - [ ] Unpack
- [ ] Execute a toml file as a command
- [ ] Registry Web API (Login, Publish and so on)
    - [ ] Login
    - [ ] Publish
    - [ ] Yank & Unyank
    - [ ] Owners
- [ ] Homepage (An Angular based SPA?)

## Dependencies

* Git

## Usage

[WIP]

## References

* Documents
    * [Git - Transfer Protocols](https://git-scm.com/book/en/v2/Git-Internals-Transfer-Protocols)
    * [Registries - The Cargo Book](https://doc.rust-lang.org/cargo/reference/registries.html)
    * 
* Projects
    * [rust-lang/crates.io-index](https://github.com/rust-lang/crates.io-index)
    * [rust-lang/crates.io](https://github.com/rust-lang/crates.io)
    * [rust-lang/cargo](https://github.com/rust-lang/cargo)
    * [AaronO/go-git-http](https://github.com/AaronO/go-git-http)
    * [tennix/crates-mirror](https://github.com/tennix/crates-mirror)
    * [rustcc/lernaean](https://github.com/rustcc/lernaean)
    * [mcorbin/meuse](https://github.com/mcorbin/meuse)
    * [samrat/rug](https://github.com/samrat/rug)
    
## License

SilkRoad is under the MIT license. See the [LICENSE](./LICENSE) file for details.
