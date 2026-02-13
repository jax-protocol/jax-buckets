# FUSE Implementation: Missing Operations

**Status:** Planned

## Objective

Implement missing FUSE filesystem operations to make the mount fully functional with standard Unix tools.

## Current Problems

When using the FUSE mount, many standard operations fail:

```bash
# These all fail with "Function not implemented"
touch file.txt
mv file.txt newname.txt
echo "content" > file.txt

# mv shows multiple errors:
# - unable to move extended attributes and ACL
# - set owner/group: Function not implemented
# - set mode: Function not implemented
# - set flags: Function not implemented
# - set times: Function not implemented
```

External editors (TextEdit, VS Code, etc.) show warnings about "permanent version storage" not being supported.

## Missing Operations

### High Priority

1. **`setattr`** - Set file attributes (mode, owner, times)
   - Required for: `touch`, `chmod`, `chown`, `mv` (setting times after copy)
   - Implementation: Update file metadata in bucket manifest

2. **`rename`** - Rename/move files within the filesystem
   - Required for: `mv`, atomic file saves
   - Implementation: Update path in bucket manifest, preserve blob hash

3. **`create`** - Create new files
   - Required for: `touch`, `echo > newfile`, editors creating new files
   - Currently partially working but inconsistent

### Medium Priority

4. **`utimens`** - Set access and modification times
   - Required for: `touch -t`, `cp -p`

5. **`truncate`/`ftruncate`** - Truncate file to specified length
   - Required for: `echo > file` (truncate before write)

### Low Priority (can return ENOTSUP)

6. **`setxattr`/`getxattr`** - Extended attributes
   - macOS uses these heavily but can be disabled

7. **`chflags`** - BSD file flags
   - macOS specific, can return ENOTSUP

## Implementation Notes

### setattr

```rust
fn setattr(&mut self, req: &Request, ino: u64, mode: Option<u32>,
           uid: Option<u32>, gid: Option<u32>, size: Option<u64>,
           atime: Option<TimeOrNow>, mtime: Option<TimeOrNow>, ...) -> Result<FileAttr>
```

For jax-bucket, we likely only need to track `mtime` since:
- Mode/uid/gid aren't meaningful for encrypted P2P storage
- Can return fixed values (0o644, current user)

### rename

```rust
fn rename(&mut self, req: &Request, parent: u64, name: &OsStr,
          newparent: u64, newname: &OsStr, flags: u32) -> Result<()>
```

Implementation:
1. Look up source entry in manifest
2. Remove from old parent directory
3. Add to new parent directory with new name
4. Preserve the blob hash (no re-upload needed)
5. Sync manifest

## Acceptance Criteria

- [ ] `touch file.txt` works
- [ ] `mv file.txt newname.txt` works
- [ ] `echo "content" > file.txt` works (for existing files)
- [ ] `cp file.txt copy.txt` preserves file properly
- [ ] External editors can save without warnings
- [ ] vim continues to work as it does now
