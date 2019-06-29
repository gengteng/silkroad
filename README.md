# SilkRoad

### A full-featured registry server for Cargo. [WIP] 

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
