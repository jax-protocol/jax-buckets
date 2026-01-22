# Role Enforcement

**Status:** Done
**Depends on:** Ticket 1 (Sync validation)

## Objective

Enforce that only peers with the `Owner` role can create bucket updates. Reject updates signed by `Mirror` peers.

## Background

The system has two roles:
- **Owner**: Full read/write access, can modify bucket contents, add/remove principals, publish
- **Mirror**: Read-only access, can sync and serve published content, cannot modify

Currently, role checking only happens client-side. A malicious mirror could sign updates and broadcast them. This ticket adds server-side enforcement during sync.

## Implementation Steps

1. **Extend `verify_provenance()` to check author role** (`crates/common/src/peer/sync/sync_bucket.rs`)

   ```rust
   fn verify_provenance<L>(peer: &Peer<L>, manifest: &Manifest) -> Result<ProvenanceResult> {
       // ... existing checks from Ticket 1 ...

       // 5. Check author has Owner role
       let author = manifest.author().expect("is_signed() was true");
       let author_hex = author.to_hex();

       let author_share = manifest.shares().get(&author_hex)
           .ok_or(ProvenanceResult::AuthorNotInShares)?;

       if *author_share.role() != PrincipalRole::Owner {
           return Ok(ProvenanceResult::AuthorNotOwner);
       }

       Ok(ProvenanceResult::Valid)
   }

   enum ProvenanceResult {
       Valid,
       NotAuthorized,
       UnsignedLegacy,
       InvalidSignature,
       AuthorNotInShares,
       AuthorNotOwner,  // NEW
   }
   ```

2. **Validate role in previous manifest** (for share/role change validation)

   When the author changes shares or roles, verify they had `Owner` role in the *previous* manifest:

   ```rust
   async fn validate_role_transition(
       previous: &Manifest,
       current: &Manifest,
   ) -> Result<(), SyncError> {
       let author = current.author().ok_or(SyncError::UnsignedManifest)?;
       let author_hex = author.to_hex();

       // Check author was Owner in previous manifest
       let prev_share = previous.shares().get(&author_hex)
           .ok_or(SyncError::AuthorNotInPrevious)?;

       if *prev_share.role() != PrincipalRole::Owner {
           return Err(SyncError::AuthorWasNotOwner);
       }

       Ok(())
   }
   ```

3. **Add role validation to chain download**

   Validate role permissions for each manifest in the chain:

   ```rust
   async fn download_manifest_chain(...) -> Result<Vec<Manifest>> {
       let mut chain = Vec::new();
       let mut previous: Option<&Manifest> = None;

       for link in links {
           let manifest = download_manifest(link).await?;

           // Basic provenance check
           verify_provenance(peer, &manifest)?;

           // Role transition check (if not genesis)
           if let Some(prev) = previous {
               validate_role_transition(prev, &manifest)?;
           }

           chain.push(manifest);
           previous = chain.last();
       }

       Ok(chain)
   }
   ```

4. **Handle sync rejection for role violations**

   ```rust
   match verify_provenance(peer, &manifest)? {
       // ... existing cases ...
       ProvenanceResult::AuthorNotOwner => {
           tracing::warn!(
               "Rejecting update from mirror {} for bucket {}",
               manifest.author().map(|a| a.to_hex()).unwrap_or_default(),
               manifest.id()
           );
           return Err(SyncError::MirrorCannotWrite);
       }
   }
   ```

5. **Add error types**

   ```rust
   pub enum SyncError {
       // ... existing variants ...
       MirrorCannotWrite,
       AuthorWasNotOwner,
       AuthorNotInPrevious,
   }
   ```

## Files to Modify

| File | Changes |
|------|---------|
| `crates/common/src/peer/sync/sync_bucket.rs` | Add role checking to `verify_provenance()`, add `validate_role_transition()` |
| `crates/common/src/peer/sync/mod.rs` | Add new error variants |

## Acceptance Criteria

- [x] `verify_provenance()` checks author has `Owner` role
- [x] Updates signed by `Mirror` peers are rejected
- [x] Role transitions are validated against previous manifest
- [x] Clear error messages for role violations
- [x] `cargo test` passes
- [x] `cargo clippy` has no warnings

## Verification

```rust
#[tokio::test]
async fn test_sync_rejects_mirror_update() {
    let owner = setup_peer().await;
    let mirror = setup_peer().await;
    let receiver = setup_peer().await;

    // Owner creates bucket, adds mirror and receiver
    let bucket = create_bucket(&owner).await;
    add_mirror(&bucket, &mirror).await;
    add_owner(&bucket, &receiver).await; // receiver can receive updates

    // Mirror tries to create an update
    let mut mount = Mount::load(&bucket.link, &mirror.secret(), &blobs).await.unwrap();
    mount.add_file("malicious.txt", b"pwned").await.unwrap();

    // Mirror signs the manifest (should be rejected)
    let (link, _, _) = mount.save(&blobs, false, &mirror.secret()).await.unwrap();

    // Receiver tries to sync - should fail
    let result = sync_bucket(&receiver, bucket.id, link).await;
    assert!(matches!(result, Err(SyncError::MirrorCannotWrite)));
}

#[tokio::test]
async fn test_role_downgrade_attack_rejected() {
    let owner = setup_peer().await;
    let attacker = setup_peer().await;
    let receiver = setup_peer().await;

    // Owner creates bucket, adds attacker as mirror
    let bucket = create_bucket(&owner).await;
    add_mirror(&bucket, &attacker).await;
    add_owner(&bucket, &receiver).await;

    // Attacker creates fake manifest upgrading themselves to owner
    let mut fake_manifest = bucket.manifest().clone();
    fake_manifest.add_share(Share::new_owner(
        SecretShare::new(&Secret::default(), &attacker.public()).unwrap(),
        attacker.public(),
    ));
    fake_manifest.sign(&attacker.secret()).unwrap();

    // Sync should fail - attacker was mirror in previous manifest
    let result = sync_bucket(&receiver, bucket.id, fake_manifest_link).await;
    assert!(matches!(result, Err(SyncError::AuthorWasNotOwner)));
}
```

## Security Considerations

1. **Genesis manifests**: The first manifest has no previous, so creator is implicitly trusted
2. **Role escalation**: Validates role in *previous* manifest to prevent self-promotion attacks
3. **Chain-of-custody**: Each manifest in the chain must be signed by someone who was Owner at that time
