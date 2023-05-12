# Verify the Release

## Import the GPG keys

Import the keys that have signed the release (if you haven’t done so already):

```
gpg --keyserver hkps://keys.openpgp.org --recv-keys 86F3105ADFA8AB587268DCD78D3DCD04249619D1
```

## Verify the manifest

```
gpg --verify coinstr-*-manifest.txt.asc
```

## Verify the binary hash

### Linux

```
sha256sum --check coinstr-*-manifest.txt --ignore-missing
```

### OSX

```
shasum --check coinstr-*-manifest.txt --ignore-missing
```

Note: Older versions of OSX (pre v11) don’t support --ignore-missing. You can leave it out and ignore the missing files reported.