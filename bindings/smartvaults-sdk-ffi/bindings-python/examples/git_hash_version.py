from smartvaults_sdk import NostrLibrary, SmartVaultsLibrary

hash = NostrLibrary().git_hash_version()
print(hash)

hash = SmartVaultsLibrary().git_hash_version()
print(hash)