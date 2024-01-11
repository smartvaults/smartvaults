from smartvaults_sdk import SmartVaults, Network, SyncHandler, init_desktop_logger
import time

init_desktop_logger("/home/user/.smartvaults", Network.TESTNET)

client = SmartVaults.open("/home/user/.smartvaults", "test", "test", Network.TESTNET)

vaults = []

class SyncNotifications(SyncHandler):
    def handle(self, msg):
        print("Refreshing...")
        global vaults
        vaults = client.vaults()

handle = client.handle_sync(SyncNotifications())

while True:
    time.sleep(5.0)

    for p in vaults:
        print(p.policy().name())

    handle.abort()
    print(f"Aborted: {handle.is_aborted()}")