from smartvaults_sdk import SmartVaults, Network, SyncHandler, init_desktop_logger
import time

init_desktop_logger("/home/user/.smartvaults", Network.TESTNET)

client = SmartVaults.restore("/home/user/.smartvaults", "name", "test", "mnemonic", None, Network.TESTNET)

class SyncNotifications(SyncHandler):
    def handle(self, msg):
        print("Refreshing...")
        policies = client.get_policies()
        for p in policies:
            print(p.policy().name())

handle = client.handle_sync(SyncNotifications())

while True:
    time.sleep(10.0)
