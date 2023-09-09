from coinstr_sdk import Coinstr, Network, SyncHandler, init_desktop_logger
import time

init_desktop_logger("/home/user/.coinstr", Network.TESTNET)

coinstr = Coinstr.restore("/home/user/.coinstr", "name", "test", "mnemonic", None, Network.TESTNET)

class SyncNotifications(SyncHandler):
    def handle(self, msg):
        print("Refreshing...")
        policies = coinstr.get_policies()
        for p in policies:
            print(p.policy().name())

handle = coinstr.handle_sync(SyncNotifications())

while True:
    time.sleep(10.0)
