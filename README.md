# SilkRoad

## Introduction

A full-featured registry server for Cargo. 

## Status

[WIP] Most features have not been implemented yet.

## RoadMap

- [ ] Serve an index repository and all crates
    - [x] HTTP server
        - [x] The Dumb Protocol
        - [x] The Smart Protocol(except git-receive-pack)
    - [ ] Git server
    - [ ] `git` related
        - [x] Command output cache
        - [ ] No dependency on `git`
    - [ ] Registry Web API (Login, Publish and so on)
        - [ ] Login
        - [ ] Publish
        - [ ] Yank & Unyank
        - [ ] Owners
- [ ] Mirroring
    - [X] Index clone and sync
    - [ ] Crates download
    - [ ] Timer
- [ ] Server Migration
    - [ ] Package
    - [ ] Unpackage
- [ ] Execute a toml file as a command
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
    * [rust-lang/crates.io-index](https://github.com/rust-lang/crates.io-index) Crates.io index.
    * [rust-lang/crates.io](https://github.com/rust-lang/crates.io) Source code for crates.io.
    * [rust-lang/cargo](https://github.com/rust-lang/cargo) The Rust package manager.
    * [AaronO/go-git-http](https://github.com/AaronO/go-git-http) A Smart Git Http server library in Go (golang).
    * [samrat/rug](https://github.com/samrat/rug) A implementation of Jit.
    * [tennix/crates-mirror](https://github.com/tennix/crates-mirror) Download all crates on Rust official crates site and keep sync with it.
    * [rustcc/lernaean](https://github.com/rustcc/lernaean) 一个面向中文社区的crates.io镜像.
    * [mcorbin/meuse](https://github.com/mcorbin/meuse) A Rust private registry written in Clojure.  
    
## License

SilkRoad is under the MIT license. See the [LICENSE](./LICENSE) file for details.
