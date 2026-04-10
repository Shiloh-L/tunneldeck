"""
Simplified E2E test for TunnelDeck's SSH tunnel functionality.
Tests against mock_ssh_server.py (port 2222) and mock_http_target.py (port 8888).
"""

import asyncio
import asyncssh
import sys


async def test_password_auth():
    """Test 1: SSH password auth."""
    print("Test 1: SSH Password Auth")
    try:
        async with asyncssh.connect(
            "127.0.0.1", 2222,
            username="test", password="test123",
            known_hosts=None,
        ) as conn:
            print("  PASS - Connected with password auth")
            return True
    except Exception as e:
        print(f"  FAIL - {e}")
        return False


async def test_kbdint_auth():
    """Test 2: Keyboard-interactive auth (Duo Push simulation)."""
    print("Test 2: Keyboard-Interactive Auth (Duo sim)")

    class DuoClient(asyncssh.SSHClient):
        def __init__(self):
            super().__init__()
            self._round = 0

        def kbdint_auth_requested(self):
            return ''

        def kbdint_challenge_received(self, name, instructions, lang, prompts):
            self._round += 1
            print(f"  Round {self._round}: name={name!r}, prompts={prompts}")
            responses = []
            for prompt_text, echo in prompts:
                prompt_lower = prompt_text.lower()
                if 'password' in prompt_lower:
                    print(f"    -> Sending password")
                    responses.append('test123')
                elif 'duo' in prompt_lower or 'push' in prompt_lower:
                    print(f"    -> Sending '1' (Duo Push approve)")
                    responses.append('1')
                else:
                    print(f"    -> Unknown prompt, sending empty")
                    responses.append('')
            return responses

    try:
        async with asyncssh.connect(
            "127.0.0.1", 2222,
            username="test",
            known_hosts=None,
            preferred_auth="keyboard-interactive",
            client_factory=DuoClient,
        ) as conn:
            print("  PASS - Keyboard-interactive auth succeeded")
            return True
    except Exception as e:
        print(f"  FAIL - {e}")
        return False


async def test_direct_tcpip():
    """Test 3: Direct-tcpip channel (port forwarding data flow)."""
    print("Test 3: Direct-tcpip Channel (Port Forward)")
    try:
        async with asyncssh.connect(
            "127.0.0.1", 2222,
            username="test", password="test123",
            known_hosts=None,
        ) as conn:
            # Open a direct-tcpip channel to the HTTP target
            reader, writer = await conn.open_connection(
                "127.0.0.1", 8888, encoding="utf-8"
            )

            # Send an HTTP request through the tunnel
            request = "GET /tunnel-test HTTP/1.0\r\nHost: 127.0.0.1:8888\r\n\r\n"
            writer.write(request)

            # Read the response
            response = ""
            while True:
                data = await asyncio.wait_for(reader.read(4096), timeout=5)
                if not data:
                    break
                response += data

            writer.close()

            if "TunnelDeck tunnel is working!" in response:
                print("  PASS - Data flows through direct-tcpip channel!")
                body_start = response.find("\r\n\r\n")
                if body_start > 0:
                    print(f"  Response body: {response[body_start+4:].strip()}")
                return True
            else:
                print(f"  FAIL - Unexpected response:\n{response[:200]}")
                return False
    except Exception as e:
        print(f"  FAIL - {e}")
        import traceback; traceback.print_exc()
        return False


async def main():
    print("=" * 50)
    print(" TunnelDeck E2E Test Suite")
    print(" SSH=127.0.0.1:2222  HTTP=127.0.0.1:8888")
    print("=" * 50)
    print()

    results = []
    results.append(await test_password_auth())
    results.append(await test_direct_tcpip())

    print()
    passed = sum(results)
    total = len(results)
    print(f"Results: {passed}/{total} passed")
    return passed == total


if __name__ == "__main__":
    ok = asyncio.run(main())
    sys.exit(0 if ok else 1)
