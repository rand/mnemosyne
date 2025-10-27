#!/usr/bin/env python3
"""Test MCP server JSON-RPC protocol"""

import subprocess
import json
import sys

def test_server():
    # Start server process
    proc = subprocess.Popen(
        ['cargo', 'run', '--quiet', '--', 'serve'],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1
    )

    tests = [
        {
            "name": "Initialize",
            "request": {"jsonrpc": "2.0", "method": "initialize", "id": 1}
        },
        {
            "name": "List Tools",
            "request": {"jsonrpc": "2.0", "method": "tools/list", "id": 2}
        },
        {
            "name": "Call Recall Tool",
            "request": {
                "jsonrpc": "2.0",
                "method": "tools/call",
                "params": {"name": "mnemosyne.recall", "arguments": {"query": "test"}},
                "id": 3
            }
        },
    ]

    try:
        for test in tests:
            print(f"\n{'='*60}")
            print(f"Test: {test['name']}")
            print(f"{'='*60}")

            # Send request
            request_str = json.dumps(test['request']) + '\n'
            print(f"Request: {request_str.strip()}")

            proc.stdin.write(request_str)
            proc.stdin.flush()

            # Read response
            response_line = proc.stdout.readline().strip()
            if response_line:
                response = json.loads(response_line)
                print(f"Response: {json.dumps(response, indent=2)}")
            else:
                print("No response received")

    finally:
        proc.terminate()
        proc.wait(timeout=2)

if __name__ == '__main__':
    test_server()
