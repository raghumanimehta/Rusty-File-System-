# Rusty-File-System-
[![Tests](https://github.com/raghumanimehta/Rusty-File-System-/actions/workflows/tests.yml/badge.svg)](https://github.com/raghumanimehta/Rusty-File-System-/actions/workflows/tests.yml)

## To Run
```
sudo mkdir /tmp/nullfs
RUST_LOG=info cargo run -- /tmp/nullfs
```

## System Dependencies
- fuse3
- libfuse3-dev
- pkg-config


### TODOs


### Main Idea

Rusty-fs is a hybrid tree-based filesystem that operates across two machines:

- **Remote Machine**: Hosts the actual filesystem state (inodes, blocks, metadata) and persists data to disk
- **Local Machine**: Mounts the filesystem via FUSE and communicates with the Remote to perform file operations

**Key Architecture:**
- Local FUSE layer translates system calls into remote operations
- Remote manages all filesystem logic, allocation, and storage
- Communication layer handles state synchronization between Local and Remote

**Completed:**
- [x] Set up Rust project structure with Cargo.toml
- [x] Implement basic FUSE filesystem mounting
- [x] Create bitmap module for inode and block allocation
- [x] Create inode data structure and management
- [x] Create filesystem metadata module
- [x] Implement filesystem state management
- [x] Add comprehensive unit tests for bitmap operations
- [x] Create basic mount test
- [x] Set up logging with env_logger
- [x] Define filesystem constants and configuration

**Not Started / In Progress:**
- [ ] Implement core FUSE filesystem operations (read, write, lookup, etc.)
- [ ] Implement directory handling and hierarchy
- [ ] Implement file creation and deletion
- [ ] Implement attribute retrieval (getattr/stat)
- [ ] Implement directory listing (readdir)
- [ ] Implement file opening and closing
- [ ] Handle file permissions and access control
- [ ] Implement data persistence to disk
- [ ] Add error handling and validation
- [ ] Implement indirect and double indirect block pointers
- [ ] Add integration tests for filesystem operations
- [ ] Optimize performance
- [ ] Document API and architecture