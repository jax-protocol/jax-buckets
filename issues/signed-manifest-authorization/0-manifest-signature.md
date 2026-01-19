# Manifest Signature

**Status:** Planned

## Objective

Add `author` and `signature` fields to `Manifest` so that each bucket update cryptographically proves who created it.

## Background

Currently, manifests have no authorship information. When a peer receives an update, there's no way to verify who created it. This ticket adds the data model changes to support signed manifests.

## Implementation Steps

1. **Add fields to Manifest struct** (`crates/common/src/mount/manifest.rs`)

   ```rust
   pub struct Manifest {
       // ... existing fields ...

       /// Public key of the peer who created this manifest version.
       /// Used to verify the signature and check role permissions.
       #[serde(default, skip_serializing_if = "Option::is_none")]
       author: Option<PublicKey>,

       /// Ed25519 signature over the manifest content (excluding this field).
       /// Signs: BLAKE3(DAG-CBOR(manifest without signature))
       #[serde(default, skip_serializing_if = "Option::is_none")]
       signature: Option<Signature>,
   }
   ```

2. **Add signing method to Manifest**

   ```rust
   impl Manifest {
       /// Sign this manifest with the given secret key.
       /// Sets author to the corresponding public key.
       pub fn sign(&mut self, secret_key: &SecretKey) -> Result<(), ManifestError> {
           self.author = Some(secret_key.public().into());
           self.signature = None; // Clear before computing

           let content = self.signable_bytes()?;
           let signature = secret_key.sign(&content);

           self.signature = Some(signature);
           Ok(())
       }

       /// Returns the bytes to sign (manifest without signature field).
       fn signable_bytes(&self) -> Result<Vec<u8>, ManifestError> {
           // Serialize manifest with signature=None
           let mut signable = self.clone();
           signable.signature = None;
           Ok(signable.to_cbor()?)
       }

       /// Verify the signature is valid for this manifest.
       pub fn verify_signature(&self) -> Result<bool, ManifestError> {
           let Some(author) = &self.author else {
               return Ok(false); // No author = unsigned
           };
           let Some(signature) = &self.signature else {
               return Ok(false); // No signature = unsigned
           };

           let content = self.signable_bytes()?;
           Ok(author.verify(&content, signature).is_ok())
       }
   }
   ```

3. **Update Mount::save() to sign manifests** (`crates/common/src/mount/mount_inner.rs`)

   The `save()` method needs to accept the owner's secret key and sign the manifest:

   ```rust
   pub async fn save(
       &self,
       blobs: &BlobsStore,
       publish: bool,
       signer: &SecretKey,  // NEW: signing key
   ) -> Result<(Link, Link, u64), MountError> {
       // ... existing save logic ...

       // Sign the manifest before storing
       manifest.sign(signer)?;

       let link = Self::_put_manifest_in_blobs(&manifest, blobs).await?;
       Ok((link, previous_link, height))
   }
   ```

4. **Add getters for new fields**

   ```rust
   impl Manifest {
       pub fn author(&self) -> Option<&PublicKey> {
           self.author.as_ref()
       }

       pub fn signature(&self) -> Option<&Signature> {
           self.signature.as_ref()
       }

       pub fn is_signed(&self) -> bool {
           self.author.is_some() && self.signature.is_some()
       }
   }
   ```

5. **Add Signature type if not present** (`crates/common/src/crypto/mod.rs`)

   Use `iroh_base::key::Signature` or wrap Ed25519 signature bytes.

## Files to Modify

| File | Changes |
|------|---------|
| `crates/common/src/mount/manifest.rs` | Add `author`, `signature` fields; add signing/verification methods |
| `crates/common/src/mount/mount_inner.rs` | Update `save()` to accept signer and sign manifest |
| `crates/common/src/peer/peer_inner.rs` | Pass secret key to `save_mount()` |

## Acceptance Criteria

- [ ] `Manifest` has `author: Option<PublicKey>` field
- [ ] `Manifest` has `signature: Option<Signature>` field
- [ ] `Manifest::sign(secret_key)` signs the manifest
- [ ] `Manifest::verify_signature()` validates the signature
- [ ] `Mount::save()` signs manifests with the owner's key
- [ ] Unsigned manifests still deserialize (backwards compatibility)
- [ ] `cargo test` passes
- [ ] `cargo clippy` has no warnings

## Verification

```rust
#[test]
fn test_manifest_signing() {
    let secret_key = SecretKey::generate();
    let mut manifest = Manifest::new(...);

    // Sign
    manifest.sign(&secret_key).unwrap();

    // Verify
    assert!(manifest.is_signed());
    assert!(manifest.verify_signature().unwrap());
    assert_eq!(manifest.author(), Some(&secret_key.public().into()));

    // Tamper detection
    manifest.set_name("tampered".to_string());
    assert!(!manifest.verify_signature().unwrap());
}
```

## Notes

- Signature covers all fields except `signature` itself
- Use BLAKE3 hash of DAG-CBOR serialized manifest for signing
- Backwards compatible: old unsigned manifests have `author=None, signature=None`
