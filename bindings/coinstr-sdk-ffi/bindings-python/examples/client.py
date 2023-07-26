from coinstr_sdk import Coinstr, Network, SyncHandler
import time

coinstr = Coinstr.open("/home/user/.coinstr", "test", "test", Network.TESTNET)

class SyncNotifications(SyncHandler):
    def handle(self):
        print("Refreshing...")

coinstr.handle_sync(SyncNotifications())

policies = coinstr.get_policies()

for p in policies:
    print(p.policy().name())


while True:
    time.sleep(5.0)