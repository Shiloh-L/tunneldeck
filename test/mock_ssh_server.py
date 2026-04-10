"""
Mock SSH server for testing TunnelDeck.
- Listens on port 2222
- Accepts password auth (user: test, password: test123)
- Supports keyboard-interactive auth (simulates Duo Push prompt)
- Allows direct-tcpip (port forwarding)
"""

import asyncio
import asyncssh
import sys


class MockSSHServer(asyncssh.SSHServer):
    """SSH server that accepts password and keyboard-interactive auth."""

    def connection_made(self, conn):
        print(f"[SSH] Connection from {conn.get_extra_info('peername')}")
        self._conn = conn

    def connection_lost(self, exc):
        if exc:
            print(f"[SSH] Connection lost: {exc}")
        else:
            print("[SSH] Connection closed")

    def begin_auth(self, username):
        # Return True to require authentication
        return True

    def password_auth_supported(self):
        return True

    def validate_password(self, username, password):
        print(f"[SSH] Password auth: user={username}")
        return username == "test" and password == "test123"

    def kbdint_auth_supported(self):
        return True

    def get_kbdint_response(self, username, lang, submethods):
        print(f"[SSH] Keyboard-interactive auth start: user={username}")
        self._kbdint_round = 0
        # First round: ask for password
        return ("TunnelDeck Auth", "",
                [("Password: ", False)])

    def validate_kbdint_response(self, username, responses):
        self._kbdint_round = getattr(self, '_kbdint_round', 0) + 1
        print(f"[SSH] Keyboard-interactive round {self._kbdint_round}, responses={responses}")

        if self._kbdint_round == 1:
            # First response should be password
            if len(responses) == 1 and responses[0] == "test123":
                print("[SSH] Password correct, sending Duo Push prompt...")
                return ("Duo Push", "",
                        [("Duo Push sent. Press Enter or type '1' to approve: ", True)])
            else:
                print(f"[SSH] Wrong password")
                return False
        elif self._kbdint_round == 2:
            # Duo Push response
            if len(responses) == 1 and responses[0] in ("", "1"):
                print("[SSH] Duo Push approved!")
                return True
            else:
                print(f"[SSH] Duo Push rejected: {responses}")
                return False
        else:
            return False

    def connection_requested(self, dest_host, dest_port, orig_host, orig_port):
        """Allow direct-tcpip (port forwarding) connections."""
        print(f"[SSH] Direct-tcpip: {orig_host}:{orig_port} -> {dest_host}:{dest_port}")
        return True


async def start_server():
    # Generate a host key
    host_key = asyncssh.generate_private_key("ssh-rsa", 2048)
    print("[SSH] Generated host key")

    await asyncssh.create_server(
        MockSSHServer,
        "",
        2222,
        server_host_keys=[host_key],
        process_factory=None,
    )

    print("=" * 50)
    print("Mock SSH Server running on port 2222")
    print("  User: test")
    print("  Password: test123")
    print("  Auth: password + keyboard-interactive (Duo sim)")
    print("  Port forwarding: enabled")
    print("=" * 50)


async def main():
    await start_server()
    # Keep running
    await asyncio.Event().wait()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n[SSH] Server stopped")
        sys.exit(0)
