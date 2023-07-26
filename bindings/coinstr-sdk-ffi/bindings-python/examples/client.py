from coinstr_sdk import Coinstr, Network, SyncHandler
import time

coinstr = Coinstr.open("/home/user/.coinstr", "test", "test", Network.TESTNET)

policies = []

class SyncNotifications(SyncHandler):
    def handle(self):
        print("Refreshing...")
        global policies
        policies = coinstr.get_policies()

handle = coinstr.handle_sync(SyncNotifications())

while True:
    time.sleep(5.0)

    for p in policies:
        print(p.policy().name())

    handle.abort()
    print(f"Aborted: {handle.is_aborted()}")