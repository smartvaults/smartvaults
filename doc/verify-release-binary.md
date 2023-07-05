# Verify the Release

## Import the GPG keys

Import the keys that have signed the release (if you haven’t done so already):

```
gpg --keyserver hkps://keys.openpgp.org --recv-keys 86F3105ADFA8AB587268DCD78D3DCD04249619D1
```

## Verify the binary

```
gpg --verify coinstr-*.asc
```